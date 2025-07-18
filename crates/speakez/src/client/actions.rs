use crate::client::UserSwitchedChannel;
use crate::common::ChannelID;
use crate::mumble::control::Message;
use crate::mumble::session::Session;

pub fn switch_channel(session: Session, from_channel: ChannelID, to_channel: ChannelID) -> Vec<u8> {
    UserSwitchedChannel {
        user: session,
        from_channel,
        to_channel,
    }
    .into_mumble()
    .as_vec()
}
