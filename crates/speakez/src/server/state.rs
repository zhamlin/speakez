use crate::common::events::UserState;
use crate::common::{Channel, User};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use bytes::BytesMut;

use crate::mumble::session::Session;
use crate::mumble::{self};

use super::handshake;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceTransport {
    Tcp,
    Udp(SocketAddr),
}

#[derive(Debug)]
pub struct SessionStats {
    pub(crate) last_seen_tcp: Instant,
    pub(crate) last_seen_udp: Option<Instant>,
}

pub struct MumbleCryptSetup {
    pub key: Vec<u8>,
    pub client_nonce: Vec<u8>,
    pub server_nonce: Vec<u8>,
}

pub trait VoiceCrypter {
    fn encrypt(&mut self, buf: &mut BytesMut);
    fn decrypt(&mut self, buf: &mut BytesMut) -> Result<(), io::Error>;
    fn crypt_setup(&self) -> MumbleCryptSetup;
}

pub struct SessionInfo {
    pub voice_transport: VoiceTransport,
    pub voice_crypter: Box<dyn VoiceCrypter>,
    pub(crate) user: User,
    pub(crate) stats: SessionStats,
}

// impl SessionInfo {
//     pub fn new(
//         voice_transport: VoiceTransport,
//         voice_crypter: Box<dyn VoiceCrypter>,
//         user: User,
//         stats: SessionStats,
//     ) -> Self {
//         Self {
//             voice_transport,
//             voice_crypter,
//             user,
//             stats,
//         }
//     }
// }

impl std::fmt::Debug for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionInfo")
            .field("voice_transport", &self.voice_transport)
            .field("user", &self.user)
            .field("stats", &self.stats)
            .finish()
    }
}

#[derive(Debug)]
pub struct Config {
    pub max_bandwidth: u32,
    pub max_users: u16,
}

#[derive(Debug, PartialEq)]
pub enum OutboxType {
    Control,
    Voice,
}

#[derive(Debug, PartialEq)]
pub enum Destination {
    All,
    AllButOne(Session),
    Single(Session),
    Group(Vec<Session>),
}

#[derive(Debug, PartialEq)]
pub enum OutboxDestination {
    Session(Destination),
    SocketAddr(SocketAddr),
}

#[derive(Debug, PartialEq)]
pub struct OutboxMessage {
    pub typ: OutboxType,
    pub data: Vec<u8>,
    pub dest: OutboxDestination,
}

#[derive(Debug)]
pub struct State {
    pub config: Config,
    pub(in crate::server) sessions: mumble::session::Sessions,
    pub(in crate::server) channels: Vec<Channel>,

    pub session_handshake: HashMap<Session, handshake::State>,
    pub session_info: HashMap<Session, SessionInfo>,
    pub socketaddr_to_session: HashMap<SocketAddr, Session>,

    pub outbox: Vec<OutboxMessage>,

    pub voice_crypter: NewVoiceCrypter,
}

impl UserState for State {
    fn get_user(&self, session: &Session) -> Option<&User> {
        self.session_info.get(session).map(|info| &info.user)
    }
}

pub type NewVoiceCrypter = fn() -> Box<dyn VoiceCrypter>;

pub fn push_message(
    messages: &mut Vec<OutboxMessage>,
    m: &impl mumble::control::Message,
    dest: Destination,
) {
    let msg = OutboxMessage {
        typ: OutboxType::Control,
        data: m.as_vec(),
        dest: OutboxDestination::Session(dest),
    };
    messages.push(msg);
}

impl State {
    pub fn new(max_users: u16, voice_crypter: NewVoiceCrypter) -> Self {
        State {
            sessions: mumble::session::Sessions::new(max_users.into()),
            config: Config {
                max_bandwidth: 480000,
                max_users,
            },
            channels: vec![],
            session_handshake: HashMap::with_capacity(max_users.into()),
            session_info: HashMap::with_capacity(max_users.into()),
            socketaddr_to_session: HashMap::with_capacity(max_users.into()),
            outbox: Vec::with_capacity(max_users.into()),
            // udp_outbox: Vec::with_capacity(max_users.into()),
            voice_crypter,
        }
    }

    pub fn new_session(&mut self) -> Option<Session> {
        self.sessions.get_session()
    }

    pub fn new_channel(&mut self, c: Channel) {
        self.channels.push(c)
    }

    pub fn push_message(&mut self, m: impl mumble::control::Message, dest: Destination) {
        push_message(&mut self.outbox, &m, dest);
    }

    pub fn push_voice_message(&mut self, m: mumble::voice::Message, dest: Destination) {
        let mut data = vec![0u8; 1024 * 4];
        let size = m.encode(&mut data).unwrap();
        data.truncate(size);

        let msg = OutboxMessage {
            typ: OutboxType::Voice,
            data,
            dest: OutboxDestination::Session(dest),
        };
        self.outbox.push(msg);
    }

    pub fn push_udp_message(&mut self, m: mumble::voice::Message, dest: SocketAddr) {
        let mut data = vec![0u8; 1024 * 4];

        let size = m.encode(&mut data).unwrap();
        data.truncate(size);

        let msg = OutboxMessage {
            typ: OutboxType::Voice,
            data,
            dest: OutboxDestination::SocketAddr(dest),
        };
        self.outbox.push(msg);
    }

    pub fn delete_session(&mut self, s: Session) -> Option<SessionInfo> {
        self.sessions.return_session(s);
        self.session_handshake.remove(&s);

        let info = self.session_info.remove(&s);

        if let Some(info) = &info {
            if let VoiceTransport::Udp(addr) = info.voice_transport {
                self.socketaddr_to_session.remove(&addr);
            }
        }

        info
    }
}
