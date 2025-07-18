// include!(concat!(env!("OUT_DIR"), "/mumble.proto.udp.rs"));
pub use crate::mumble::proto::voice::*;

// The maximum allowed size in bytes of UDP packets (according to the Mumble protocol)
pub const MAX_UDP_PACKET_SIZE: usize = 1024;

pub fn message_length<T: prost::Message>(m: &T) -> usize {
    m.encoded_len()
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Audio = 0,
    Ping = 1,
}

#[derive(Debug, PartialEq)]
pub enum Message {
    Audio(Audio),
    Ping(Ping),
}

impl Message {
    pub fn typ(&self) -> MessageType {
        match self {
            Message::Audio(_) => MessageType::Audio,
            Message::Ping(_) => MessageType::Ping,
        }
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, prost::EncodeError> {
        use prost::Message as _;

        macro_rules! encode {
            ($m:ident) => {{
                let len = $m.encoded_len();
                let mut msg_body = &mut buf[1..len + 1];
                $m.encode(&mut msg_body)?;

                len
            }};
        }

        let msg_size = match self {
            Message::Audio(m) => encode!(m),
            Message::Ping(m) => encode!(m),
        };

        buf[0] = self
            .typ()
            .to_u16()
            .try_into()
            .expect("message type should not have more than 255 values");
        Ok(msg_size + 1)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        use prost::Message as _;

        // TODO: move this piece out?
        let typ_byte = buf[0];
        let typ = MessageType::from_u16(typ_byte.into()).ok_or(prost::DecodeError::new(
            format!("invalid message type, found: {}", typ_byte),
        ))?;

        match typ {
            MessageType::Audio => {
                let msg = Audio::decode(&buf[1..])?;
                Ok(Message::Audio(msg))
            }
            MessageType::Ping => {
                let msg = Ping::decode(&buf[1..])?;
                Ok(Message::Ping(msg))
            }
        }
    }
}

impl MessageType {
    pub fn to_u16(self) -> u16 {
        self as u16
    }

    pub fn from_u16(n: u16) -> Option<Self> {
        let kind = match n {
            // region:sourcegen
            0 => MessageType::Audio,
            1 => MessageType::Ping,
            // endregion:sourcegen
            _ => return None,
        };
        Some(kind)
    }
}

#[test]
fn sourcegen_from_code() {
    use gen::enums::RegionGenerator;
    use std::fmt::Write;

    let generators = vec![RegionGenerator::new(
        "sourcegen",
        |variants, indent, name| {
            variants.iter().fold(String::new(), |mut output, v| {
                let _ = writeln!(output, "{indent}{} => {}::{},", v.value, name, v.name);
                output
            })
        },
    )];

    let file_path = std::path::Path::new(file!().strip_prefix("crates/speakez/").unwrap());
    gen::enums::sourcegen_from_code(file_path, "MessageType", 12, &generators);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_audio() {
        let audio = Audio {
            sender_session: 1,
            frame_number: 2,
            is_terminator: true,
            header: Some(audio::Header::Context(0)),
            ..Default::default()
        };
        let msg = Message::Audio(audio);
        let mut buf = vec![0u8; 1024];
        let size = msg.encode(&mut buf).unwrap();

        let new_msg = Message::decode(&buf[..size]).unwrap();
        assert_eq!(msg, new_msg);
    }
}
