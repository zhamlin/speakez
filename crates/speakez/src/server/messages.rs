use prost::Message as _;

use crate::common::events::{self, mumble_to_event, Event};
use std::net::SocketAddr;
use std::time::Instant;

use crate::mumble::control::{Message as _, MessageBuf};
use crate::mumble::session::Session;
use crate::mumble::{self, control, voice};

use super::handshake::handle_handshake;
use super::state::{
    Destination, OutboxDestination, OutboxMessage, OutboxType, State, VoiceTransport,
};
use super::{handshake, version};

#[derive(Debug)]
pub enum Message {
    Tick,
    SessionCreated(Session),
    SessionDisconnect(Session),
    Mumble(Session, MessageBuf),
    UDP(SocketAddr, Vec<u8>),
}

fn handle_session_disconnect(mut s: State, session: Session) -> State {
    let user = match s.delete_session(session) {
        Some(info) => info.user,
        None => return s,
    };

    let event = events::UserRemoved {
        user: session,
        reason: events::UserRemovedReason::Left,
        reason_msg: None,
    };
    s.push_message(event.into_mumble(), Destination::AllButOne(user.session));

    s
}

// #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
fn handle_voice_message(mut s: State, session: Session, msg: events::VoiceMessage) -> State {
    let audio_msg = mumble::voice::Message::Audio(msg.into());
    s.push_voice_message(audio_msg, Destination::AllButOne(session));

    s
}

fn handle_udp_unencrypted_ping(
    mut s: State,
    from: SocketAddr,
    m: mumble::voice::Ping,
    now: Instant,
) -> State {
    let ping = mumble::voice::Ping {
        timestamp: m.timestamp,
        ..Default::default()
    };

    let msg = mumble::voice::Message::Ping(ping);
    s.push_udp_message(msg, from);
    s
}

fn handle_udp_ping(mut s: State, session: Session, m: mumble::voice::Ping, now: Instant) -> State {
    let ping = mumble::voice::Ping {
        timestamp: m.timestamp,
        ..Default::default()
    };

    let msg = mumble::voice::Message::Ping(ping);
    s.push_voice_message(msg, Destination::Single(session));
    s
}

/// Returns the first session found that has a matching SocketAddr.
fn find_matching_addr(s: &State, from: SocketAddr) -> Option<Session> {
    // TODO: Switch to hashmap to store a sessions SocketAddr, prevent multiple Sessions having the
    // same SocketAddr.
    s.session_info
        .iter()
        .find_map(|(session, info)| match info.voice_transport {
            VoiceTransport::Udp(addr) if addr == from => Some(*session),
            _ => None,
        })
}

/// Find a session that can decrypt and decode the provided data.
/// If the message is decoded succesfully, return the message and matching session.
fn find_matching_crypt(s: &mut State, data: &[u8]) -> Option<(Session, voice::Message)> {
    // find all sessions without a matching SocketAddr
    // and try to decrypt+decode the message to find a match
    let item = s.session_info.iter_mut().find_map(|(session, info)| {
        if info.voice_transport != VoiceTransport::Tcp {
            return None;
        }

        let mut b = bytes::BytesMut::from(data);
        // TODO: does the state change on invalid decrypt attempts?
        if info.voice_crypter.decrypt(&mut b).is_err() {
            return None;
        }

        match mumble::voice::Message::decode(&b) {
            Ok(m) => Some((*session, m)),
            Err(_) => None,
        }
    });
    item
}

fn maybe_decode_unencrypted_ping(data: &[u8]) -> Option<voice::Ping> {
    match mumble::voice::Message::decode(data) {
        Ok(m) => match m {
            mumble::voice::Message::Audio(_) => {
                // TODO: err to client
                crate::tracing::error!("audio packets must encrypted");
                None
            }
            mumble::voice::Message::Ping(m) => Some(m),
        },
        Err(err) => {
            // This could have failed for a few reasons:
            // 1. Packet is encrypted
            // 2. It is the legacy format for UDP packets (not protobuf)
            // 3. Invalid packet
            crate::tracing::debug!("UDP packet unknown format; possibly old UDP format");
            println!("{}", err);
            None
        }
    }
}

/// UDP message could be one of the following:
/// - unencrypted ping packet
/// - encrypted ping/audio packet
// #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
fn handle_udp_message(mut s: State, from: SocketAddr, data: Vec<u8>, now: Instant) -> State {
    if let Some(session) = find_matching_addr(&s, from) {
        crate::tracing::debug!("found existing upd session, {:#?}", session);
        let info = s
            .session_info
            .get_mut(&session)
            .expect("session should have session info");

        let mut b = bytes::BytesMut::from(&data[..]);
        info.voice_crypter.decrypt(&mut b).unwrap();

        let msg = match mumble::voice::Message::decode(&b) {
            Ok(m) => m,
            Err(_) => todo!("error to client"),
        };

        match msg {
            mumble::voice::Message::Audio(mut a) => {
                if a.sender_session == 0 {
                    a.sender_session = session.into();
                }

                let msg = events::mumble_voice_to_event(a);
                return handle_voice_message(s, session, msg);
            }
            mumble::voice::Message::Ping(p) => return handle_udp_ping(s, session, p, now),
        };
    }

    if let Some((session, msg)) = find_matching_crypt(&mut s, &data) {
        crate::tracing::debug!("found matching crypt, {:#?}", session);
        let info = s
            .session_info
            .get_mut(&session)
            .expect("session should have session info");

        info.voice_transport = VoiceTransport::Udp(from);

        match msg {
            mumble::voice::Message::Audio(mut a) => {
                if a.sender_session == 0 {
                    a.sender_session = session.into();
                }
                let msg = events::mumble_voice_to_event(a);
                return handle_voice_message(s, session, msg);
            }
            mumble::voice::Message::Ping(p) => return handle_udp_ping(s, session, p, now),
        }
    }

    if let Some(ping) = maybe_decode_unencrypted_ping(&data) {
        crate::tracing::debug!("unencrypted ping, {:#?}", from);
        return handle_udp_unencrypted_ping(s, from, ping, now);
    }

    s
}

fn handle_event(mut s: State, session: Session, e: Event) -> State {
    match e {
        Event::UserSentAudio(e) => {
            return handle_voice_message(s, session, e);
        }
        Event::UserSwitchedChannel(e) => {
            let info = s.session_info.get_mut(&e.user).unwrap();
            if info.user.channel == e.from_channel {
                info.user.channel = e.to_channel;
                s.push_message(e.into_mumble(), Destination::All);
            }
        }
        Event::UserRemoved(user_removed) => todo!(),
        Event::UserJoinedServer(user_joined_server) => todo!(),
        Event::UserSentMessage(m) => {
            let msg = OutboxMessage {
                typ: OutboxType::Control,
                data: m.into_mumble().as_vec(),
                dest: OutboxDestination::Session(Destination::AllButOne(session)),
            };
            s.outbox.push(msg);
        }
    }

    s
}

fn handle_mumble_message(
    mut s: State,
    session: Session,
    m: MessageBuf,
    msg_received_at: Instant,
) -> State {
    if let Some(hs) = s.session_handshake.remove(&session) {
        return handle_handshake(s, hs, session, m, msg_received_at);
    }

    if let Some(event) = mumble_to_event(&s, &m, Some(session)) {
        return handle_event(s, session, event);
    }

    match m.typ {
        control::MessageType::Ping => {
            let p = control::proto::Ping::decode(m.body()).unwrap();
            let ping = control::proto::Ping {
                good: p.good,
                ..Default::default()
            };
            s.push_message(ping, Destination::Single(session));
        }
        control::MessageType::PermissionQuery => {
            let q = control::proto::PermissionQuery::decode(m.body()).unwrap();
            let msg = control::proto::PermissionQuery {
                channel_id: q.channel_id,
                permissions: Some(mumble::permissions::default()),
                ..Default::default()
            };
            s.push_message(msg, Destination::Single(session));
        }
        control::MessageType::TextMessage => {
            let msg = OutboxMessage {
                typ: OutboxType::Control,
                data: m.data.clone(),
                dest: OutboxDestination::Session(Destination::AllButOne(session)),
            };
            s.outbox.push(msg);
        }
        control::MessageType::CryptSetup => todo!("Handle Crypt Resync"),
        control::MessageType::UDPTunnel => unreachable!("UDPTunnel should be handled separately"),
        typ => {
            crate::tracing::info!("unhandled mumble message: {:#?}", typ);
        }
    };

    s
}

/// Send the server version to the client and add the session to the state.
fn handle_session_new(mut s: State, session: Session) -> State {
    let hs = handshake::State::new(session);
    s.session_handshake.insert(session, hs);

    s.push_message(version(), Destination::Single(session));

    s
}

fn handle_tick(s: State, now: Instant) -> State {
    // TODO: Create timer for: Close client connection if no ping within 30 seconds
    s
}

// TODO: How to force disconnect from server::state

// #[instrument(skip(s, now, m))]
pub fn handle_message(s: State, m: Message, now: Instant) -> State {
    match m {
        Message::SessionCreated(session) => handle_session_new(s, session),
        Message::SessionDisconnect(session) => handle_session_disconnect(s, session),
        Message::Mumble(session, m) => handle_mumble_message(s, session, m, now),
        Message::UDP(from, data) => handle_udp_message(s, from, data, now),
        Message::Tick => handle_tick(s, now),
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::common::{self, Channel};
    use crate::server::state::{MumbleCryptSetup, OutboxDestination, VoiceCrypter};

    use self::mumble::voice;

    use super::*;

    fn udp_audio_message_to_buf(m: voice::Audio) -> Vec<u8> {
        let mut buf = vec![0u8; 1024];
        let len = mumble::voice::Message::Audio(m).encode(&mut buf).unwrap();
        buf[..len].to_vec()
    }

    fn udp_ping_message_to_buf(m: voice::Ping) -> Vec<u8> {
        let mut buf = vec![0u8; 1024];
        let len = mumble::voice::Message::Ping(m).encode(&mut buf).unwrap();
        buf[..len].to_vec()
    }

    fn message_to_buf(m: impl control::Message) -> MessageBuf {
        MessageBuf {
            typ: m.message_type(),
            data: m.as_vec(),
        }
    }

    fn perform_handshake(mut s: State, username: String) -> (State, Session) {
        let session = s.new_session().unwrap();
        let auth = control::proto::Authenticate {
            username: Some(username),
            password: Some("password".to_string()),
            ..Default::default()
        };

        let now = Instant::now();
        let s = vec![
            Message::SessionCreated(session),
            Message::Mumble(session, message_to_buf(version())),
            Message::Mumble(session, message_to_buf(auth)),
        ]
        .into_iter()
        .fold(s, |s, m| handle_message(s, m, now));

        (s, session)
    }

    struct TestVoiceCrypter {
        key: Vec<u8>,
        client_nonce: Vec<u8>,
        server_nonce: Vec<u8>,
    }

    impl Default for TestVoiceCrypter {
        fn default() -> Self {
            Self {
                key: vec![1u8; 16],
                client_nonce: vec![0u8; 16],
                server_nonce: vec![0u8; 16],
            }
        }
    }

    impl VoiceCrypter for TestVoiceCrypter {
        fn encrypt(&mut self, _: &mut bytes::BytesMut) {}

        fn decrypt(&mut self, _: &mut bytes::BytesMut) -> Result<(), std::io::Error> {
            Ok(())
        }

        fn crypt_setup(&self) -> MumbleCryptSetup {
            MumbleCryptSetup {
                key: self.key.clone(),
                client_nonce: self.client_nonce.clone(),
                server_nonce: self.server_nonce.clone(),
            }
        }
    }

    fn new_state(max_users: u16) -> State {
        State::new(max_users, || Box::new(TestVoiceCrypter::default()))
    }

    fn want_message<M: control::Message + Default + PartialEq>(
        msg: M,
        dest: Destination,
        got: OutboxMessage,
    ) {
        let got_message = M::decode(&got.data[control::proto::PREFIX_TOTAL_SIZE..]).unwrap();
        assert_eq!(
            (msg, OutboxDestination::Session(dest)),
            (got_message, got.dest)
        );
    }

    #[test]
    fn test_handshake() {
        let mut s = new_state(10);
        let channel = Channel {
            id: common::ROOT_CHANNEL,
            name: "TestChannel".to_string(),
            description: "Description".to_string(),
            temporary: false,
            max_users: None,
            position: None,
            parent: None,
        };
        s.new_channel(channel.clone());

        let username = "username".to_string();
        let (mut s, session) = perform_handshake(s, username.clone());

        s.outbox.reverse();
        let mut next = || s.outbox.pop().unwrap();

        want_message(version(), Destination::Single(session), next());
        want_message(
            control::proto::CryptSetup {
                key: Some(vec![1u8; 16]),
                client_nonce: Some(vec![0u8; 16]),
                server_nonce: Some(vec![0u8; 16]),
            },
            Destination::Single(session),
            next(),
        );
        want_message(
            control::proto::ChannelState {
                channel_id: Some(0),
                name: Some(channel.name),
                description: Some(channel.description),
                ..Default::default()
            },
            Destination::Single(session),
            next(),
        );
        want_message(
            control::proto::UserState {
                session: Some(session.into()),
                name: Some(username.clone()),
                channel_id: Some(channel.id.into()),
                ..Default::default()
            },
            Destination::Single(session),
            next(),
        );
        want_message(
            control::proto::ServerSync {
                session: Some(session.into()),
                max_bandwidth: Some(s.config.max_bandwidth),
                welcome_text: Some("Hello Test user".to_string()),
                permissions: Some(mumble::permissions::default() as u64),
            },
            Destination::Single(session),
            next(),
        );
        // message broadcasting the new user has joined
        want_message(
            control::proto::UserState {
                session: Some(session.into()),
                name: Some(username.clone()),
                channel_id: Some(channel.id.into()),
                ..Default::default()
            },
            Destination::AllButOne(session),
            next(),
        );

        assert_eq!(s.outbox.pop(), None);
    }

    #[test]
    fn test_upd_unencrypted_ping() {
        let packet = {
            udp_ping_message_to_buf(mumble::voice::Ping {
                timestamp: 1,
                ..Default::default()
            })
        };

        let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let m = Message::UDP(addr, packet.clone());

        let s = new_state(1);
        let mut s = handle_message(s, m, Instant::now());

        let item = s.outbox.pop().expect("should have a ping packet");
        assert_eq!(
            (item.data, item.dest),
            (packet, OutboxDestination::SocketAddr(addr))
        );
    }

    #[test]
    fn test_err_upd_unencrypted_audio() {
        let packet = {
            udp_audio_message_to_buf(mumble::voice::Audio {
                ..Default::default()
            })
        };

        let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let m = Message::UDP(addr, packet);

        let s = new_state(1);
        let mut s = handle_message(s, m, Instant::now());
        // TODO: check for error to client
        assert_eq!(s.outbox.pop(), None);
    }

    #[test]
    fn test_udp_encrypted() {
        let mut s = new_state(10);
        let channel = Channel {
            id: common::ROOT_CHANNEL,
            name: "TestChannel".to_string(),
            description: "Description".to_string(),
            temporary: false,
            max_users: None,
            position: None,
            parent: None,
        };
        s.new_channel(channel.clone());

        let username = "username".to_string();
        let (mut s, session) = perform_handshake(s, username.clone());

        s.outbox.reverse();
        s.outbox.pop(); // version
        let m = s
            .outbox
            .pop()
            .expect("should have CryptSetup after version");
        s.outbox.drain(..);

        let msg = {
            let (typ, _) = control::parse_prefix(&m.data[..control::proto::PREFIX_TOTAL_SIZE]);
            assert!(matches!(typ, control::MessageType::CryptSetup));
            control::proto::CryptSetup::decode(&m.data[control::proto::PREFIX_TOTAL_SIZE..])
                .unwrap()
        };

        let packet = {
            udp_ping_message_to_buf(mumble::voice::Ping {
                timestamp: 1,
                ..Default::default()
            })
        };

        let mut crypt = TestVoiceCrypter {
            key: msg.key.unwrap(),
            client_nonce: msg.client_nonce.unwrap(),
            server_nonce: msg.server_nonce.unwrap(),
        };

        let mut extened_packet: Vec<u8> = Vec::with_capacity(packet.len() + 4);
        extened_packet.append(&mut packet.clone());
        let mut b = bytes::BytesMut::from(&extened_packet[..]);
        // TODO: modify encrypt to handle the header size
        crypt.encrypt(&mut b);

        let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let m = Message::UDP(addr, b.to_vec());
        let mut s = handle_message(s, m, Instant::now());
        let item = s.outbox.pop().expect("should have a ping packet");

        // TODO: decrypt response
        assert_eq!(
            (item.data, item.dest),
            (
                packet,
                OutboxDestination::Session(Destination::Single(session))
            )
        );
        assert_eq!(s.session_info.len(), 1);
    }
}
