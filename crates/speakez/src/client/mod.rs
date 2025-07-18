pub mod actions;
pub mod handshake;
pub mod voice;

use std::collections::HashMap;

use crate::common::events::{self, Event, UserSwitchedChannel};
use crate::common::{Channel, ChannelID, User};
use crate::mumble::control::MessageBuf;
use crate::mumble::session::Session;
use crate::mumble::{self, control};

pub fn version() -> control::proto::Version {
    let v = mumble::Version::new(1, 5, 0);
    control::proto::Version {
        os: Some("mumbleWeb".to_string()),
        release: Some(v.to_string()),
        version_v2: Some(v.to_u64()),
        ..Default::default()
    }
}

/// Created via a HandshakeState.
#[derive(Clone, Debug)]
pub struct State {
    pub session: Session,
    pub users: HashMap<Session, User>,
    pub channels: HashMap<ChannelID, Channel>,

    pub outbox: Vec<Event>,
}

impl State {
    fn new(session: Session) -> Self {
        Self {
            session,
            users: HashMap::new(),
            channels: HashMap::new(),
            outbox: vec![],
        }
    }

    pub fn get_self(&self) -> &User {
        self.users
            .get(&self.session)
            .expect("should have self in users list")
    }
}

impl events::UserState for State {
    fn get_user(&self, session: &Session) -> Option<&User> {
        self.users.get(session)
    }
}

pub enum Message {
    Mumble(MessageBuf),
}

pub fn handle_message(s: State, m: Message) -> State {
    match m {
        Message::Mumble(m) => handle_mumble_message(s, m),
    }
}

fn mumble_to_event<S: events::UserState>(s: &S, m: &MessageBuf) -> Option<Event> {
    events::mumble_to_event(s, m, None)
}

pub fn handle_mumble_message(s: State, m: MessageBuf) -> State {
    match mumble_to_event(&s, &m) {
        Some(e) => handle_event(s, e),
        None => s,
    }
}

pub fn handle_event(mut s: State, e: Event) -> State {
    match e {
        Event::UserRemoved(ref event) => {
            s.users.remove(&event.user);
        }
        Event::UserSwitchedChannel(ref event) => {
            let user = s
                .users
                .get_mut(&event.user)
                .expect("should have user if received UserSwitchedChannel event");
            user.channel = event.to_channel;
        }
        Event::UserJoinedServer(ref event) => {
            let user = User {
                name: event.name.clone(),
                session: event.user,
                channel: event.channel_id,
            };
            s.users.insert(event.user, user);
        }
        _ => (),
    };

    s.outbox.push(e);
    s
}
