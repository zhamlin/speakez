use speakez_client::{audio::DeviceConfig, commands, Client, ClientRef, Config};
use std::sync::Arc;

fn init_subscriber() {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;
    let subscriber = FmtSubscriber::builder()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn main() {
    init_subscriber();

    tracing::info!("speakez client starting");
    run();
    tracing::info!("speakez client shutting down");
}

fn get_tls_config() -> rustls::ClientConfig {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    provider
        .install_default()
        .expect("provider failed to install as the default");

    use rustls_platform_verifier::BuilderVerifierExt;
    let mut config = rustls::ClientConfig::builder()
        .with_platform_verifier()
        .with_no_client_auth();

    let verifier = speakez_client::tls::danger::NoCertificateVerification::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    );

    config
        .dangerous()
        .set_certificate_verifier(Arc::new(verifier));

    config
}

fn run() {
    let sample_rate = 48000;
    let frame_size = libopus::calc_frame_size(sample_rate, 10);

    let cfg = Config {
        latency: 150.0,
        input: DeviceConfig {
            channels: 1,
            name: None,
            sample_rate,
            buf_size: frame_size,
        },
        output: DeviceConfig {
            channels: 2,
            name: None,
            sample_rate,
            buf_size: 512,
        },
    };

    let delay = std::time::Duration::from_secs(60 * 10);
    println!("Playing for {delay:?} seconds... ");

    let config = get_tls_config();
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut c = Client::new(
        config,
        Arc::new(move |msg| {
            sender.send(msg).unwrap();
            Ok(())
        }),
    );

    let client = ClientRef::from(&c);
    std::thread::Builder::new()
        .name("client".to_string())
        .spawn(move || {
            while let Ok(msg) = receiver.recv() {
                c.handle_message(msg);

                if let Some(events) = c.events() {
                    for event in events {
                        dbg!(event);
                    }
                }

                for resp in c.responses() {
                    dbg!(resp);
                }
            }
        })
        .unwrap();

    client.input_set_device(cfg.input);
    client.output_set_device(cfg.output);

    let addr = std::env::var("MUMBLE_HOST").unwrap_or("127.0.0.1:64738".to_string());
    client.connect(commands::Connect {
        addr,
        user: "cli_test_user".into(),
        pass: "".into(),
    });

    std::thread::sleep(delay);
}
