mod handshake;
mod messages;
pub mod state;

pub use messages::{handle_message, Message};
pub use state::Destination;

use crate::mumble::{self, control};

pub fn version() -> control::proto::Version {
    let v = mumble::Version::new(1, 5, 0);
    control::proto::Version {
        os: Some("testOS".to_string()),
        release: Some(v.to_string()),
        version_v2: Some(v.to_u64()),
        ..Default::default()
    }
}
