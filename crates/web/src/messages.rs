use speakez::mumble::control::{proto, Message};
use speakez::mumble::{self, control};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version_message() -> Vec<u8> {
    speakez::server::version().as_vec()
}

#[wasm_bindgen]
pub fn authenticate_message(name: String, password: String) -> Vec<u8> {
    proto::Authenticate {
        username: Some(name),
        password: Some(password),
        opus: Some(true),
        ..Default::default()
    }
    .as_vec()
}

#[wasm_bindgen]
pub fn tunneled_opus_message(
    sender: u32,
    frame_number: u64,
    opus_data: Vec<u8>,
    is_terminator: bool,
) -> Vec<u8> {
    let mumble_audio = mumble::voice::Audio {
        sender_session: sender,
        opus_data,
        frame_number,
        // This is set by the server
        volume_adjustment: 0.0,
        is_terminator,
        ..Default::default()
    };

    // 1 for the message type
    let length = 1 + mumble::voice::message_length(&mumble_audio);
    let mut buf = vec![0u8; proto::PREFIX_TOTAL_SIZE + length];

    mumble::control::write_message_header(control::MessageType::UDPTunnel, length, &mut buf[..]);

    let msg = speakez::mumble::voice::Message::Audio(mumble_audio);
    msg.encode(&mut buf[proto::PREFIX_TOTAL_SIZE..]).unwrap();

    buf
}

#[wasm_bindgen]
pub fn opus_message(
    sender: u32,
    frame_number: u64,
    opus_data: Vec<u8>,
    is_terminator: bool,
) -> Vec<u8> {
    // positional_data: todo!(),
    // header: todo!(),
    let mumble_audio = mumble::voice::Audio {
        sender_session: sender,
        opus_data,
        frame_number,
        // This is set by the server
        volume_adjustment: 0.0,
        is_terminator,
        ..Default::default()
    };

    let length = mumble::voice::message_length(&mumble_audio);
    let mut buf = vec![0u8; length + proto::PREFIX_TOTAL_SIZE + 1];

    let msg = speakez::mumble::voice::Message::Audio(mumble_audio);
    msg.encode(&mut buf).unwrap();
    buf
}
