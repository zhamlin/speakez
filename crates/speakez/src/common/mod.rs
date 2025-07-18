pub mod events;

use std::num::{NonZeroI32, NonZeroU32};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::mumble::session::Session;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    // TODO: remove?
    pub session: Session,
    pub channel: ChannelID,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChannelID(u32);

impl ChannelID {
    pub fn new(id: u32) -> Self {
        ChannelID(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Into<u32> for ChannelID {
    fn into(self) -> u32 {
        self.0
    }
}

pub const ROOT_CHANNEL: ChannelID = ChannelID(0);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug)]
pub struct Channel {
    pub id: ChannelID,
    pub name: String,
    pub description: String,
    pub temporary: bool,
    pub max_users: Option<NonZeroU32>,
    pub position: Option<NonZeroI32>,
    pub parent: Option<ChannelID>,
    // TODO: description_hash
}

impl Channel {
    pub fn new(
        id: ChannelID,
        name: String,
        description: String,
        temporary: bool,
        max_users: Option<NonZeroU32>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            temporary,
            max_users,
            position: None,
            parent: None,
        }
    }
}
