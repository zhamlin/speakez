pub mod client {
    use crate::mumble::control::proto;
    use crate::mumble::control::MessageBuf;
    use crate::mumble::Version;
    use crate::mumble::{self, control};

    use prost::Message;

    #[derive(Clone, Debug)]
    pub struct ServerVersion {
        pub version: Version,
    }

    #[derive(Clone, Debug)]
    pub struct ServerState {
        pub(crate) crypt: proto::CryptSetup,
        pub(crate) channels: Vec<proto::ChannelState>,
        pub(crate) users: Vec<proto::UserState>,
    }

    #[derive(Clone, Debug)]
    pub struct ServerSync {
        pub(crate) state: ServerState,
        pub(crate) sync: proto::ServerSync,
    }

    #[derive(Clone, Debug)]
    pub enum State {
        /// Connected to the server and version has been sent.
        Connected,
        /// The servers version message has been received.
        ServerVersion(ServerVersion),
        /// Authenticate message has been sent to the server.
        SentAuthenticate,
        /// CryptSetup message has been received but the ServerSync
        /// message has not.
        StateSync(ServerState),
        /// ServerSync message has been received.
        ServerSync(ServerSync),
    }

    impl State {
        pub fn new() -> Self {
            State::Connected
        }

        fn handle_version(self, msg: proto::Version) -> Self {
            let version = mumble::Version::from_u64(msg.version_v2());
            State::ServerVersion(ServerVersion { version })
        }

        fn handle_crypt_setup(self, msg: proto::CryptSetup) -> Self {
            let state = ServerState {
                crypt: msg,
                users: Vec::new(),
                channels: Vec::new(),
            };
            State::StateSync(state)
        }

        pub fn handle(self, m: MessageBuf) -> Self {
            match self {
                State::Connected if m.typ == control::MessageType::Version => {
                    let msg = proto::Version::decode(m.body()).unwrap();
                    self.handle_version(msg)
                }
                // allow sending of authenticate without waiting for the server version,
                State::SentAuthenticate if m.typ == control::MessageType::Version => self,
                State::SentAuthenticate if m.typ == control::MessageType::CryptSetup => {
                    let msg = control::proto::CryptSetup::decode(m.body()).unwrap();
                    self.handle_crypt_setup(msg)
                }
                State::StateSync(mut s) if m.typ == control::MessageType::ChannelState => {
                    let msg = control::proto::ChannelState::decode(m.body()).unwrap();
                    s.channels.push(msg);
                    State::StateSync(s)
                }
                State::StateSync(mut s) if m.typ == control::MessageType::UserState => {
                    let msg = control::proto::UserState::decode(m.body()).unwrap();
                    s.users.push(msg);
                    State::StateSync(s)
                }
                State::StateSync(state) if m.typ == control::MessageType::ServerSync => {
                    let msg = control::proto::ServerSync::decode(m.body()).unwrap();
                    let sync = ServerSync { state, sync: msg };
                    State::ServerSync(sync)
                }
                _ => todo!("client handshake: {:?}", m.typ),
            }
        }
    }
}

pub mod server {
    use prost::Message;

    use crate::mumble::control::MessageBuf;
    use crate::mumble::Version;
    use crate::mumble::{self, control};

    #[derive(Clone, Debug)]
    pub enum AuthMethod {
        Password(String),
        Cert(Vec<u8>),
    }

    #[derive(Clone, Debug)]
    pub struct Authentication {
        pub username: String,
        pub method: AuthMethod,
    }

    #[derive(Clone, Debug)]
    pub struct ClientVersion {
        pub(crate) version: Version,
    }

    #[derive(Debug)]
    pub struct ClientAuth {
        pub(crate) version: ClientVersion,
        pub(crate) auth: Authentication,
    }

    #[derive(Clone, Debug)]
    pub enum State {
        SentServerVersion,
        ClientVersion(ClientVersion),
        Authenticate(Authentication),
    }

    impl State {
        pub fn new() -> Self {
            State::SentServerVersion
        }

        fn handle_version(&mut self, msg: mumble::control::proto::Version) {
            let version = mumble::Version::from_u64(msg.version_v2());
            *self = State::ClientVersion(ClientVersion { version })
        }

        fn handle_authenticate(&mut self, msg: mumble::control::proto::Authenticate) {
            let auth = Authentication {
                username: msg.username().to_string(),
                method: AuthMethod::Password(msg.password.unwrap()),
            };
            *self = State::Authenticate(auth)
        }

        pub fn handle(&mut self, m: MessageBuf) {
            match self {
                State::SentServerVersion if m.typ == control::MessageType::Version => {
                    let msg = control::proto::Version::decode(m.body()).unwrap();
                    self.handle_version(msg)
                }
                State::ClientVersion(_) if m.typ == control::MessageType::Authenticate => {
                    let msg = control::proto::Authenticate::decode(m.body()).unwrap();
                    self.handle_authenticate(msg)
                }
                got => todo!("got {:?}, in {:?} state", m.typ, got),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mumble;
    use crate::mumble::control::{proto, Message, MessageBuf, MessageType};

    use super::*;

    pub fn version_message() -> Vec<u8> {
        let v = mumble::Version::new(1, 5, 0);
        proto::Version {
            os: Some("testOS".to_string()),
            release: Some(v.to_string()),
            version_v2: Some(v.to_u64()),
            ..Default::default()
        }
        .as_vec()
    }

    pub fn crypt_message() -> Vec<u8> {
        proto::CryptSetup {
            ..Default::default()
        }
        .as_vec()
    }

    pub fn channel_message() -> Vec<u8> {
        proto::ChannelState {
            ..Default::default()
        }
        .as_vec()
    }

    #[test]
    fn test_client_handshake() {
        let s = client::State::SentAuthenticate;

        // let msg = MessageBuf {
        //     typ: MessageType::Version,
        //     data: version_message(),
        // };
        // s = s.handle(m)
        let msgs = vec![
            MessageBuf {
                typ: MessageType::CryptSetup,
                data: crypt_message(),
            },
            MessageBuf {
                typ: MessageType::ChannelState,
                data: channel_message(),
            },
        ];

        let s = msgs.into_iter().fold(s, |s, m| s.handle(m));
        eprintln!("{:?}", s);
    }
}
