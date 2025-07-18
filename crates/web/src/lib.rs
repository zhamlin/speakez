pub mod handshake;
pub mod messages;

use speakez::client::{self};
use speakez::common::{Channel, ChannelID, User};
use speakez::mumble::control;
use speakez::mumble::session::Session;

use self::handshake::HandshakeWrapper;

use wasm_bindgen::prelude::*;

// Manually additional types generated from jsonschema.
#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = include_str!("../schemas/speakez.d.ts");

// This function will be called when the WebAssembly module is initialized.
#[cfg(feature = "panic_hook")]
#[wasm_bindgen(start)]
pub fn main_js() {
    use std::panic;
    // Set the panic hook to provide better error messages in the console
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u64(a: u64);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_usize(a: usize);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

#[wasm_bindgen]
pub struct MessageBufWrapper(control::MessageBuf);

#[wasm_bindgen]
impl MessageBufWrapper {
    #[wasm_bindgen]
    pub fn typ(&self) -> u16 {
        self.0.typ.to_u16()
    }

    #[wasm_bindgen]
    pub fn typ_to_string(&self) -> String {
        self.0.typ.as_str().to_string()
    }
}

#[wasm_bindgen]
pub fn new_message_buf(data: Vec<u8>) -> Option<MessageBufWrapper> {
    let prefix = control::get_prefix_from_buf(&data)?;
    let (typ, size) = control::parse_prefix(prefix);
    let buf = control::MessageBuf { typ, data };
    Some(MessageBufWrapper(buf))
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct StateWrapper(client::State);

#[wasm_bindgen]
pub fn switch_channel(s: &StateWrapper, channel_id: u32) -> Vec<u8> {
    let current_channel = s.0.get_self().channel;
    let to_channel = ChannelID::new(channel_id);
    // TODO: Remove from client and move to this packge
    client::actions::switch_channel(s.0.session, current_channel, to_channel)
}

#[wasm_bindgen]
impl StateWrapper {
    #[wasm_bindgen]
    pub fn next_event(&mut self) -> JsValue {
        match self.0.outbox.pop() {
            Some(e) => serde_wasm_bindgen::to_value(&e).unwrap(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn session(&mut self) -> u32 {
        self.0.session.into()
    }

    #[wasm_bindgen]
    pub fn channel(&mut self, id: u32) -> JsValue {
        match self.0.channels.get(&ChannelID::new(id)) {
            Some(e) => serde_wasm_bindgen::to_value(&e).unwrap(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn channels(&mut self) -> JsValue {
        serde_wasm_bindgen::to_value(
            &self
                .0
                .channels
                .values()
                .map(|v| v.to_owned())
                .collect::<Vec<Channel>>(),
        )
        .unwrap()
    }

    #[wasm_bindgen]
    pub fn user(&mut self, id: u32) -> JsValue {
        let session = match Session::new(id) {
            Some(s) => s,
            None => return JsValue::NULL,
        };

        match self.0.users.get(&session) {
            Some(e) => serde_wasm_bindgen::to_value(&e).unwrap(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn users(&mut self) -> JsValue {
        serde_wasm_bindgen::to_value(
            &self
                .0
                .users
                .values()
                .map(|v| v.to_owned())
                .collect::<Vec<User>>(),
        )
        .unwrap()
    }
}

#[wasm_bindgen]
pub fn handle_message(mut s: StateWrapper, msg: MessageBufWrapper) -> StateWrapper {
    s.0 = client::handle_mumble_message(s.0, msg.0);
    s
}

#[wasm_bindgen]
pub fn new_connection() -> HandshakeWrapper {
    HandshakeWrapper::new()
}
