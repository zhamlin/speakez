use speakez::client::handshake::Status;
use speakez::client::{self};
use speakez::mumble::handshake;

use wasm_bindgen::prelude::*;

use crate::{MessageBufWrapper, StateWrapper};

#[wasm_bindgen]
#[derive(Debug)]
pub struct HandshakeWrapper(client::handshake::State);

#[wasm_bindgen]
#[derive(Debug)]
pub struct HandshakeResult {
    pub typ: u8,
    status: Status,
}

const HANDSHAKE: u8 = 1;
const CONNECTED: u8 = 2;

#[wasm_bindgen]
impl HandshakeResult {
    #[wasm_bindgen]
    pub fn is_handshake(&self) -> bool {
        self.typ == HANDSHAKE
    }

    #[wasm_bindgen]
    pub fn to_handshake(self) -> HandshakeWrapper {
        match self.status {
            Status::Handshake(state) => HandshakeWrapper(state),
            Status::Connected(_) => {
                panic!("to_handshake must be called when is_handshake is true");
            }
        }
    }

    #[wasm_bindgen]
    pub fn is_connected(&self) -> bool {
        self.typ == CONNECTED
    }

    #[wasm_bindgen]
    pub fn to_connected(self) -> StateWrapper {
        match self.status {
            Status::Handshake(_) => {
                panic!("to_connected must be called when is_connected is true");
            }
            Status::Connected(state) => StateWrapper(state),
        }
    }
}

#[wasm_bindgen]
impl HandshakeWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        HandshakeWrapper(client::handshake::State::new())
    }

    #[wasm_bindgen]
    pub fn handle_message(self, m: MessageBufWrapper) -> HandshakeResult {
        match self.0.handle_message(m.0) {
            client::handshake::Status::Handshake(handshake_state) => HandshakeResult {
                typ: HANDSHAKE,
                status: Status::Handshake(handshake_state),
            },
            client::handshake::Status::Connected(state) => HandshakeResult {
                typ: CONNECTED,
                status: Status::Connected(state),
            },
        }
    }

    #[wasm_bindgen]
    pub fn should_send_authenticate(&mut self) -> bool {
        matches!(self.0.state, handshake::client::State::ServerVersion(_))
    }

    #[wasm_bindgen]
    pub fn sent_authenticate(&mut self) {
        match &mut self.0.state {
            handshake::client::State::ServerVersion(_) => {
                self.0.state = handshake::client::State::SentAuthenticate
            }
            state => todo!("client should not send auth in {:?} state", state),
        }
    }
}
