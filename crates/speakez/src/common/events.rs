use prost::Message;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::mumble;
use crate::mumble::control::{self, MessageBuf};
use crate::mumble::session::Session;

use super::{ChannelID, User, ROOT_CHANNEL};

/// Only opus is supported.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct VoiceMessage {
    pub data: Vec<u8>,
    pub frame_number: u64,
    pub sender: Session,
}

impl From<VoiceMessage> for mumble::voice::Audio {
    fn from(msg: VoiceMessage) -> Self {
        mumble::voice::Audio {
            opus_data: msg.data,
            frame_number: msg.frame_number,
            // is_terminator: incoming_audio.is_terminator,
            sender_session: msg.sender.into(),
            ..Default::default()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub enum UserRemovedReason {
    /// The user left the server.
    Left,
    /// The user was kicked from the server.
    Kicked {
        /// The user who initiated the removal.
        by: Session,
    },
    /// The user was banned from the server.
    Banned {
        /// The user who initiated the removal.
        by: Session,
    },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct UserRemoved {
    /// the user who was removed.
    pub user: Session,
    pub reason: UserRemovedReason,
    pub reason_msg: Option<String>,
}

impl UserRemoved {
    pub fn into_mumble(self) -> control::proto::UserRemove {
        self.into()
    }
}

impl From<UserRemoved> for control::proto::UserRemove {
    fn from(value: UserRemoved) -> Self {
        let mut msg = control::proto::UserRemove {
            session: value.user.into(),
            reason: value.reason_msg,
            actor: None,
            ban: None,
        };

        match value.reason {
            UserRemovedReason::Left => {}
            UserRemovedReason::Kicked { by } => {
                msg.actor = Some(by.into());
            }
            UserRemovedReason::Banned { by } => {
                msg.actor = Some(by.into());
                msg.ban = Some(true)
            }
        }

        msg
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct UserSwitchedChannel {
    /// The user who switched channels.
    pub user: Session,
    pub from_channel: ChannelID,
    pub to_channel: ChannelID,
}

impl UserSwitchedChannel {
    pub fn into_mumble(self) -> control::proto::UserState {
        self.into()
    }
}

impl From<UserSwitchedChannel> for control::proto::UserState {
    fn from(val: UserSwitchedChannel) -> Self {
        control::proto::UserState {
            session: Some(val.user.into()),
            actor: Some(val.user.into()),
            channel_id: Some(val.to_channel.into()),
            ..Default::default()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct UserJoinedServer {
    /// The user who joined.
    pub user: Session,
    pub name: String,
    pub channel_id: ChannelID,
}

impl From<UserJoinedServer> for control::proto::UserState {
    fn from(value: UserJoinedServer) -> Self {
        control::proto::UserState {
            name: Some(value.name),
            session: Some(value.user.into()),
            channel_id: Some(value.channel_id.into()),
            ..Default::default()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct UserSentMessage {
    /// The user who sent the message.
    pub user: Session,
    /// Users who should receive them message.
    pub recipients: Vec<Session>,
    /// Channels that should receive the message.
    pub channels: Vec<ChannelID>,
    pub message: String,
}

impl UserSentMessage {
    pub fn into_mumble(self) -> control::proto::TextMessage {
        self.into()
    }
}

impl From<UserSentMessage> for control::proto::TextMessage {
    fn from(value: UserSentMessage) -> Self {
        control::proto::TextMessage {
            actor: Some(value.user.into()),
            session: value.recipients.into_iter().map(|s| s.into()).collect(),
            channel_id: value.channels.into_iter().map(|c| c.into()).collect(),
            message: value.message,
            ..Default::default()
        }
    }
}

// Event naming: Subject Verb Object
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "data"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub enum Event {
    UserSentAudio(VoiceMessage),
    UserSentMessage(UserSentMessage),
    UserRemoved(UserRemoved),
    UserSwitchedChannel(UserSwitchedChannel),
    UserJoinedServer(UserJoinedServer),
}

pub fn mumble_voice_to_event(audio: mumble::voice::Audio) -> VoiceMessage {
    let sender = Session::new(audio.sender_session).unwrap();
    VoiceMessage {
        data: audio.opus_data,
        frame_number: audio.frame_number,
        sender,
    }
}

fn mumble_text_to_event(e: control::proto::TextMessage) -> UserSentMessage {
    let session = e.actor.and_then(Session::new).unwrap();
    UserSentMessage {
        user: session,
        channels: e.channel_id.into_iter().map(ChannelID::new).collect(),
        recipients: e
            .session
            .into_iter()
            .map(|s| Session::new(s).unwrap())
            .collect(),
        message: e.message,
    }
}

fn mumble_user_remove_to_event(e: control::proto::UserRemove) -> UserRemoved {
    let session = Session::new(e.session).unwrap();
    let reason = match (e.actor, e.ban()) {
        (Some(actor), true) => {
            let by = Session::new(actor).unwrap();
            UserRemovedReason::Banned { by }
        }
        (Some(actor), false) => {
            let by = Session::new(actor).unwrap();
            UserRemovedReason::Kicked { by }
        }
        (_, _) => UserRemovedReason::Left,
    };

    UserRemoved {
        user: session,
        reason,
        reason_msg: e.reason,
    }
}

pub trait UserState {
    fn get_user(&self, session: &Session) -> Option<&User>;
}

fn mumble_user_state_to_event(s: &impl UserState, e: control::proto::UserState) -> Option<Event> {
    let session = Session::new(e.session.unwrap()).unwrap();

    let user = match s.get_user(&session) {
        Some(u) => u,
        None => {
            // TODO: unwrap_or(ROOT_CHANNEL)?
            let channel_id = ChannelID::new(e.channel_id.unwrap());
            let event = UserJoinedServer {
                user: session,
                name: e.name.unwrap(),
                channel_id,
            };
            return Some(Event::UserJoinedServer(event));
        }
    };

    match (user.channel, e.channel_id.map(ChannelID::new)) {
        (current, Some(new)) if current != new => {
            let msg = UserSwitchedChannel {
                user: session,
                from_channel: current,
                to_channel: new,
            };
            return Some(Event::UserSwitchedChannel(msg));
        }
        _ => {}
    }

    // TODO:
    // 1. User Muted/Unmuted

    dbg!(e);
    None
}

fn mumble_audio_to_event(m: &MessageBuf, sender: Option<Session>) -> Option<VoiceMessage> {
    let msg = mumble::voice::Message::decode(m.body()).unwrap();
    match msg {
        mumble::voice::Message::Audio(mut audio) => {
            if audio.sender_session == 0 {
                audio.sender_session = sender.map_or(0, |s| s.into());
            }
            Some(mumble_voice_to_event(audio))
        }
        mumble::voice::Message::Ping(_) => None,
    }
}

pub fn mumble_to_event<S: UserState>(
    s: &S,
    m: &MessageBuf,
    sender: Option<Session>,
) -> Option<Event> {
    if m.typ == control::MessageType::UDPTunnel {
        return mumble_audio_to_event(m, sender).map(Event::UserSentAudio);
    }

    match m.typ {
        control::MessageType::UserState => {
            let e = control::proto::UserState::decode(m.body()).unwrap();
            e.session?;
            mumble_user_state_to_event(s, e)
        }
        control::MessageType::UserRemove => {
            let e = control::proto::UserRemove::decode(m.body()).unwrap();
            let e = mumble_user_remove_to_event(e);
            Some(Event::UserRemoved(e))
        }
        control::MessageType::TextMessage => {
            let mut e = control::proto::TextMessage::decode(m.body()).unwrap();
            if e.actor.is_none() {
                e.actor = sender.map(|s| s.into())
            }

            let e = mumble_text_to_event(e);
            Some(Event::UserSentMessage(e))
        }
        control::MessageType::UDPTunnel => unreachable!("UDP tunnel handled above"),
        _ => None,
    }
}
