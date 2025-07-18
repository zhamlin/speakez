// https://v2.tauri.app/develop/calling-rust/

mod client;
mod commands;

use std::time::Duration;

use client::ClientRef;
use tauri::{Emitter, Manager};
use tokio::sync::broadcast;

fn get_tls_config() -> rustls::ClientConfig {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    provider.install_default().unwrap();

    let verifier = speakez_client::tls::danger::NoCertificateVerification::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    );

    use rustls_platform_verifier::BuilderVerifierExt;
    let mut config = rustls::ClientConfig::builder()
        .with_platform_verifier()
        .with_no_client_auth();

    config
        .dangerous()
        .set_certificate_verifier(std::sync::Arc::new(verifier));

    config
}

struct AppState {
    client: ClientRef,
}

fn setup(app: &mut tauri::App) {
    let config = get_tls_config();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let tokio_sender = sender.clone();
    let sender = std::sync::Arc::new(move |msg| {
        sender.send(msg).unwrap();
        Ok(())
    });
    let s = sender.clone();
    let mut client = speakez_client::Client::new(config, s);

    let (tx, _) = broadcast::channel(16);
    let client_ref = ClientRef::new(sender, tx.clone());
    let app_state = AppState { client: client_ref };
    app.manage(app_state);

    let handle = app.app_handle().clone();
    let client_loop = move || {
        while let Some(msg) = receiver.blocking_recv() {
            client.handle_message(msg);

            if let Some(events) = client.events() {
                for event in events {
                    handle.emit("mumble", event).unwrap();
                }
            }

            for response in client.responses() {
                tx.send(response).unwrap();
            }
        }
    };

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            let instant = ticker.tick().await;
            let e = speakez_client::Event::Tick();
            tokio_sender
                .send(speakez_client::Message::Event(e))
                .unwrap();
        }
    });

    tauri::async_runtime::spawn_blocking(client_loop);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::set_input,
            commands::set_output,
            commands::connect,
            commands::disconnect,
            commands::switch_channel,
            commands::mic_mute,
            commands::mic_monitor,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // only include this code on debug builds
            #[cfg(debug_assertions)]
            {
                let w = app.get_webview_window("main").unwrap();
                w.open_devtools();
            }

            setup(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
