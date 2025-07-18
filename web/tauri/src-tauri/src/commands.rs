use crate::AppState;
use speakez_client::{
    audio::DeviceConfig,
    commands::{self, response},
    speakez::common::ChannelID,
};
use tauri::AppHandle;

#[tauri::command]
pub fn switch_channel(state: tauri::State<AppState>, channel_id: u32) {
    let id = ChannelID::new(channel_id);
    state.client.switch_channel(id);
}

#[tauri::command]
pub fn set_input(state: tauri::State<AppState>) {
    let sample_rate = 48000;
    let frame_size = speakez_client::audio::calc_frame_size(sample_rate, 10);
    let input = DeviceConfig {
        channels: 1,
        name: None,
        sample_rate,
        buf_size: frame_size,
    };
    let client = &state.client;
    client.input_set_device(input);
}

#[tauri::command]
pub fn set_output(app: AppHandle, state: tauri::State<AppState>) {
    let sample_rate = 48000;
    let output = DeviceConfig {
        channels: 2,
        name: None,
        sample_rate,
        buf_size: 512,
    };
    let client = &state.client;
    client.output_set_device(output);
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConnectResp {
    message: String,
}

#[tauri::command]
pub async fn connect(
    state: tauri::State<'_, AppState>,
    url: String,
    user: String,
    pass: String,
) -> Result<response::Connect, ConnectResp> {
    state
        .client
        .connect(commands::Connect {
            addr: url,
            user,
            pass,
        })
        .await
        .map_err(|e| ConnectResp { message: e })
}

#[tauri::command]
pub async fn disconnect(state: tauri::State<'_, AppState>) -> Result<response::Disconnect, String> {
    state.client.disconnect().await
}

#[tauri::command]
pub fn mic_monitor(state: tauri::State<'_, AppState>, value: bool) {
    dbg!(value);
    state.client.input_monitor(value);
}

#[tauri::command]
pub fn mic_mute(state: tauri::State<'_, AppState>, value: bool) {
    dbg!(value);
    state.client.input_mute(value);
}
