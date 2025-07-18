use std::time::Instant;

use divan::{black_box, AllocProfiler, Bencher};
use speakez::server::state::{MumbleCryptSetup, VoiceCrypter};
use speakez::server::Message;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

struct TestVoiceCrypter {
    key: Vec<u8>,
    client_nonce: Vec<u8>,
    server_nonce: Vec<u8>,
}

impl Default for TestVoiceCrypter {
    fn default() -> Self {
        Self {
            key: vec![1u8; 16],
            client_nonce: vec![0u8; 16],
            server_nonce: vec![0u8; 16],
        }
    }
}

impl VoiceCrypter for TestVoiceCrypter {
    fn encrypt(&mut self, _: &mut bytes::BytesMut) {}

    fn decrypt(&mut self, _: &mut bytes::BytesMut) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn crypt_setup(&self) -> MumbleCryptSetup {
        MumbleCryptSetup {
            key: self.key.clone(),
            client_nonce: self.client_nonce.clone(),
            server_nonce: self.server_nonce.clone(),
        }
    }
}

fn new_state(max_users: u16) -> speakez::server::state::State {
    speakez::server::state::State::new(max_users, || Box::new(TestVoiceCrypter::default()))
}

#[divan::bench(sample_count = 1000)]
fn bench_handshake_session_created(bencher: Bencher) {
    let now = Instant::now();
    bencher
        .with_inputs(|| {
            let mut s = new_state(1);
            let session = s.new_session().unwrap();

            (s, Message::SessionCreated(session))
        })
        .bench_values(|(s, msg)| black_box(speakez::server::handle_message(s, msg, now)));
}
