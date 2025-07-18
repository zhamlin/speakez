use std::num::NonZeroI32;

use crate::common::{Channel, ChannelID, User};
use crate::mumble::control::{self, MessageBuf};
use crate::mumble::handshake;
use crate::mumble::session::Session;

use super::State as ClientState;

#[derive(Clone, Debug)]
pub enum Status {
    Handshake(State),
    Connected(ClientState),
}

/// State used during the initial handshake.
#[derive(Clone, Debug)]
pub struct State {
    pub state: handshake::client::State,
}

impl State {
    pub fn new() -> Self {
        Self {
            state: handshake::client::State::new(),
        }
    }

    pub fn handle_message(mut self, m: MessageBuf) -> Status {
        self.state = self.state.handle(m);
        match self.state {
            handshake::client::State::ServerSync(data) => {
                let session_id = data.sync.session.unwrap();
                let session = Session::new(session_id).unwrap();

                let mut state = ClientState::new(session);
                for user in data.state.users {
                    let session = Session::new(user.session.unwrap()).unwrap();
                    let channel = ChannelID::new(user.channel_id.unwrap());
                    let u = User {
                        name: user.name.unwrap(),
                        session,
                        channel,
                    };
                    state.users.insert(session, u);
                }

                for channel in data.state.channels {
                    let id = ChannelID::new(channel.channel_id.unwrap());
                    let control::proto::ChannelState {
                        name,
                        description,
                        temporary,
                        position,
                        parent,
                        ..
                    } = channel;

                    let c = Channel {
                        id,
                        name: name.unwrap(),
                        description: description.unwrap(),
                        temporary: temporary.unwrap_or(false),
                        max_users: None,
                        position: position.map(|i| NonZeroI32::new(i).unwrap()),
                        parent: parent.map(ChannelID::new),
                    };

                    state.channels.insert(id, c);
                }

                Status::Connected(state)
            }
            _ => Status::Handshake(self),
        }
    }
}
