use std::time::Instant;

use crate::common::{events, User, ROOT_CHANNEL};

use super::state::{push_message, SessionInfo, SessionStats, State as ServerState, VoiceTransport};
use super::Destination;
use crate::mumble::control::{self, MessageBuf};
use crate::mumble::session::Session;
use crate::mumble::{self, handshake};

#[derive(Debug)]
pub enum Status {
    Handshake(State),
    Connected(User),
}

/// State used during the initial handshake.
#[derive(Clone, Debug)]
pub struct State {
    pub state: handshake::server::State,
    pub session: Session,
}

impl State {
    pub fn new(session: Session) -> Self {
        Self {
            state: handshake::server::State::new(),
            session,
        }
    }

    pub fn handle_message(mut self, m: MessageBuf) -> Status {
        self.state.handle(m);
        match self.state {
            handshake::server::State::Authenticate(auth) => {
                let u = User {
                    name: auth.username,
                    session: self.session,
                    channel: ROOT_CHANNEL,
                };
                Status::Connected(u)
            }
            _ => Status::Handshake(self),
        }
    }
}

pub(super) fn handle_handshake(
    mut s: ServerState,
    hs: State,
    session: Session,
    m: MessageBuf,
    msg_received_at: Instant,
) -> ServerState {
    match hs.handle_message(m) {
        Status::Handshake(state) => {
            s.session_handshake.insert(session, state);
        }
        Status::Connected(user) => {
            let info = new_session_info(&s, user, msg_received_at);
            handle_session_connected(&mut s, info);
        }
    }
    s
}

fn new_session_info(s: &ServerState, user: User, msg_received_at: Instant) -> SessionInfo {
    SessionInfo {
        voice_transport: VoiceTransport::Tcp,
        voice_crypter: (s.voice_crypter)(),
        user,
        stats: SessionStats {
            last_seen_tcp: msg_received_at,
            last_seen_udp: None,
        },
    }
}

fn handle_session_connected(s: &mut ServerState, info: SessionInfo) {
    let session = info.user.session;
    let msg = events::UserJoinedServer {
        name: info.user.name.clone(),
        user: session,
        channel_id: ROOT_CHANNEL,
    }
    .into();

    sync_server_state_to_session(s, &info, &msg);
    s.session_info.insert(session, info);
    s.push_message(msg, Destination::AllButOne(session));
}

/// This function assumes the user_state is not in the state already.
fn sync_server_state_to_session(
    s: &mut ServerState,
    info: &SessionInfo,
    user_state: &control::proto::UserState,
) {
    let session = info.user.session;
    let msg = {
        let crypt = info.voice_crypter.crypt_setup();
        control::proto::CryptSetup {
            key: Some(crypt.key),
            client_nonce: Some(crypt.client_nonce),
            server_nonce: Some(crypt.server_nonce),
        }
    };
    s.push_message(msg, Destination::Single(session));

    let channel_states = s
        .channels
        .iter()
        .map(|channel| control::proto::ChannelState {
            name: Some(channel.name.clone()),
            description: Some(channel.description.clone()),
            channel_id: Some(channel.id.as_u32()),
            position: channel.position.map(|i| i.into()),
            parent: if channel.id == ROOT_CHANNEL {
                None
            } else {
                Some(ROOT_CHANNEL.into())
            },
            ..Default::default()
        });

    for msg in channel_states {
        push_message(&mut s.outbox, &msg, Destination::Single(session));
    }

    let user_states = s
        .session_info
        .values()
        .map(|info| control::proto::UserState {
            name: Some(info.user.name.clone()),
            session: Some(info.user.session.into()),
            channel_id: Some(info.user.channel.into()),
            ..Default::default()
        });

    for msg in user_states {
        push_message(&mut s.outbox, &msg, Destination::Single(session));
    }
    push_message(&mut s.outbox, user_state, Destination::Single(session));

    let msg = control::proto::ServerSync {
        session: Some(session.into()),
        welcome_text: Some("Hello Test user".to_string()),
        max_bandwidth: Some(s.config.max_bandwidth),
        permissions: Some(mumble::permissions::default().into()),
    };
    s.push_message(msg, Destination::Single(session));
}
