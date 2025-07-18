use prost::Message as ProtoMessage;

pub mod proto {
    const MiB: u32 = 1024_u32.pow(2);
    pub const MAX_MSG_SIZE: u32 = (8 * MiB) - 1;
    pub const PREFIX_TYPE_SIZE: usize = 2;
    pub const PREFIX_LEN_SIZE: usize = 4;
    pub const PREFIX_TOTAL_SIZE: usize = PREFIX_TYPE_SIZE + PREFIX_LEN_SIZE;

    pub use crate::mumble::proto::control::*;
    // include!(concat!(env!("OUT_DIR"), "/mumble.proto.rs"));
}

pub fn get_prefix_from_buf(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() >= proto::PREFIX_TOTAL_SIZE {
        Some(&buf[0..proto::PREFIX_TOTAL_SIZE])
    } else {
        None
    }
}

pub fn write_message_header(typ: MessageType, length: usize, buf: &mut [u8]) -> usize {
    let total_length = proto::PREFIX_TOTAL_SIZE + length;
    assert!(buf.len() >= total_length);

    let msg_type_be = u16::to_be_bytes(typ.to_u16());
    buf[0..proto::PREFIX_TYPE_SIZE].copy_from_slice(&msg_type_be);

    let msg_size_be = u32::to_be_bytes(length.try_into().unwrap());
    buf[proto::PREFIX_TYPE_SIZE..proto::PREFIX_TOTAL_SIZE].copy_from_slice(&msg_size_be);

    total_length
}

pub fn encode_udp_tunnel(data: &[u8], buf: &mut [u8]) -> usize {
    let total_length = write_message_header(MessageType::UDPTunnel, data.len(), buf);

    let msg_body = &mut buf[proto::PREFIX_TOTAL_SIZE..total_length];
    msg_body[..].copy_from_slice(data);

    total_length
}

pub fn encode_message(m: &impl Message, buf: &mut [u8]) -> usize {
    let length = message_length(m);
    let total_length = write_message_header(m.message_type(), length, buf);

    let mut msg_body = &mut buf[proto::PREFIX_TOTAL_SIZE..total_length];
    m.encode(&mut msg_body).unwrap();

    total_length
}

pub fn total_message_length(m: &impl ProtoMessage) -> usize {
    proto::PREFIX_TOTAL_SIZE + message_length(m)
}

pub fn message_length(m: &impl ProtoMessage) -> usize {
    m.encoded_len()
}

pub fn parse_prefix(buf: &[u8]) -> (MessageType, usize) {
    assert_eq!(buf.len(), proto::PREFIX_TOTAL_SIZE);
    let msg_type = u16::from_be_bytes(buf[0..proto::PREFIX_TYPE_SIZE].try_into().unwrap());
    let msg_len = u32::from_be_bytes(
        buf[proto::PREFIX_TYPE_SIZE..proto::PREFIX_TOTAL_SIZE]
            .try_into()
            .unwrap(),
    );

    (
        MessageType::from_u16(msg_type).unwrap(),
        msg_len.try_into().unwrap(),
    )
}

#[derive(Debug)]
pub struct MessageBuf {
    pub typ: MessageType,
    pub data: Vec<u8>,
}

impl MessageBuf {
    /// Return a reference to the data without the prefix.
    pub fn body(&self) -> &[u8] {
        &self.data[proto::PREFIX_TOTAL_SIZE..]
    }
}

pub trait Message: ProtoMessage + TypedMessage + Sized {
    fn as_vec(&self) -> Vec<u8> {
        let length = total_message_length(self);
        let mut buf = vec![0u8; length];
        encode_message(self, &mut buf[..]);
        buf
    }
}

impl<T> Message for T where T: ProtoMessage + TypedMessage {}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Version = 0,
    UDPTunnel = 1,
    Authenticate = 2,
    Ping = 3,
    Reject = 4,
    ServerSync = 5,
    ChannelRemove = 6,
    ChannelState = 7,
    UserRemove = 8,
    UserState = 9,
    BanList = 10,
    TextMessage = 11,
    PermissionDenied = 12,
    ACL = 13,
    QueryUsers = 14,
    CryptSetup = 15,
    ContextActionModify = 16,
    ContextAction = 17,
    UserList = 18,
    VoiceTarget = 19,
    PermissionQuery = 20,
    CodecVersion = 21,
    UserStats = 22,
    RequestBlob = 23,
    ServerConfig = 24,
    SuggestConfig = 25,
}

impl MessageType {
    pub fn to_u16(self) -> u16 {
        self as u16
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            // region:enum_str
            MessageType::Version => "Version",
            MessageType::UDPTunnel => "UDPTunnel",
            MessageType::Authenticate => "Authenticate",
            MessageType::Ping => "Ping",
            MessageType::Reject => "Reject",
            MessageType::ServerSync => "ServerSync",
            MessageType::ChannelRemove => "ChannelRemove",
            MessageType::ChannelState => "ChannelState",
            MessageType::UserRemove => "UserRemove",
            MessageType::UserState => "UserState",
            MessageType::BanList => "BanList",
            MessageType::TextMessage => "TextMessage",
            MessageType::PermissionDenied => "PermissionDenied",
            MessageType::ACL => "ACL",
            MessageType::QueryUsers => "QueryUsers",
            MessageType::CryptSetup => "CryptSetup",
            MessageType::ContextActionModify => "ContextActionModify",
            MessageType::ContextAction => "ContextAction",
            MessageType::UserList => "UserList",
            MessageType::VoiceTarget => "VoiceTarget",
            MessageType::PermissionQuery => "PermissionQuery",
            MessageType::CodecVersion => "CodecVersion",
            MessageType::UserStats => "UserStats",
            MessageType::RequestBlob => "RequestBlob",
            MessageType::ServerConfig => "ServerConfig",
            MessageType::SuggestConfig => "SuggestConfig",
            // endregion:enum_str
        }
    }

    pub fn from_u16(n: u16) -> Option<MessageType> {
        let kind = match n {
            // region:sourcegen
            0 => MessageType::Version,
            1 => MessageType::UDPTunnel,
            2 => MessageType::Authenticate,
            3 => MessageType::Ping,
            4 => MessageType::Reject,
            5 => MessageType::ServerSync,
            6 => MessageType::ChannelRemove,
            7 => MessageType::ChannelState,
            8 => MessageType::UserRemove,
            9 => MessageType::UserState,
            10 => MessageType::BanList,
            11 => MessageType::TextMessage,
            12 => MessageType::PermissionDenied,
            13 => MessageType::ACL,
            14 => MessageType::QueryUsers,
            15 => MessageType::CryptSetup,
            16 => MessageType::ContextActionModify,
            17 => MessageType::ContextAction,
            18 => MessageType::UserList,
            19 => MessageType::VoiceTarget,
            20 => MessageType::PermissionQuery,
            21 => MessageType::CodecVersion,
            22 => MessageType::UserStats,
            23 => MessageType::RequestBlob,
            24 => MessageType::ServerConfig,
            25 => MessageType::SuggestConfig,
            // endregion:sourcegen
            _ => return None,
        };
        Some(kind)
    }
}

pub trait TypedMessage {
    fn message_type(&self) -> MessageType;
}

macro_rules! typed_message {
    ($type:ty, $msg_type:expr) => {
        impl TypedMessage for $type {
            fn message_type(&self) -> MessageType {
                $msg_type
            }
        }
    };
}

macro_rules! typed_messages {
    ($(($type:ty, $msg_type:expr)),* $(,)?) => {
        $(
            typed_message!($type, $msg_type);
        )*
    };
}

typed_messages!(
    (proto::Ping, MessageType::Ping),
    (proto::Version, MessageType::Version),
    (proto::ChannelState, MessageType::ChannelState),
    (proto::UserState, MessageType::UserState),
    (proto::UserRemove, MessageType::UserRemove),
    (proto::TextMessage, MessageType::TextMessage),
    (proto::PermissionQuery, MessageType::PermissionQuery),
    (proto::ServerSync, MessageType::ServerSync),
    (proto::Authenticate, MessageType::Authenticate),
    (proto::CryptSetup, MessageType::CryptSetup),
);

// https://matklad.github.io/2022/03/26/self-modifying-code.html
#[test]
fn sourcegen_from_code() {
    use gen::enums::RegionGenerator;
    use std::fmt::Write;

    let generators = vec![
        RegionGenerator::new("sourcegen", |variants, indent, name| {
            variants.iter().fold(String::new(), |mut output, v| {
                let _ = writeln!(output, "{indent}{} => {}::{},", v.value, name, v.name);
                output
            })
        }),
        RegionGenerator::new("enum_str", |variants, indent, name| {
            variants.iter().fold(String::new(), |mut output, v| {
                let _ = writeln!(output, r#"{indent}{}::{} => "{}","#, name, v.name, v.name);
                output
            })
        }),
    ];

    let file_path = std::path::Path::new(file!().strip_prefix("crates/speakez/").unwrap());
    gen::enums::sourcegen_from_code(file_path, "MessageType", 12, &generators);
}
