/*
 * Copied from: https://github.com/Johni0702/rust-mumble-protocol/blob/master/src/crypt.rs
 * Also used https://github.com/SOZ-Faut-etre-Sub/ZUMBLE/blob/oss/src/crypt.rs as reference for
 * rust aes.
 * Modified for use in this project.

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.

*/

//! Implementation of the cryptography used for Mumble's voice channel

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;

use aws_lc_rs::constant_time::verify_slices_are_equal;
use aws_lc_rs::rand::{SecureRandom, SystemRandom};

use bytes::BytesMut;
use std::convert::TryInto;
use std::io;
use std::sync::LazyLock;

use speakez::server::state::{MumbleCryptSetup, VoiceCrypter};

// NOTE: static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
static SYSTEM_RANDOM: LazyLock<SystemRandom> = LazyLock::new(aws_lc_rs::rand::SystemRandom::new);

pub fn fill(dest: &mut [u8]) -> Result<(), ()> {
    SYSTEM_RANDOM.fill(dest).map_err(|_| ())
}

/// Maximum size of an encrypted Mumble packet.
/// Note that larger packets can be produced if there is sufficient voice data in one packet but
/// there's no guarantee that the remote end will not just drop it.
pub const MAX_PACKET_SIZE: usize = 1024;
/// Size in bytes of the AES key used in `CryptState`.
pub const KEY_SIZE: usize = 16;
/// Size in bytes of blocks for the AES primitive.
pub const BLOCK_SIZE: usize = std::mem::size_of::<u128>();

/// Implements OCB2-AES128 for encryption and authentication of the voice packets
/// when transmitted over UDP.
/// Also provides statistics about good, late and lost packets.
///
/// Implements a `Codec` which parses a stream of encrypted data chunks into [VoicePacket]s.
///
/// Note that OCB is covered by patents, however a license has been granted for use in "most"
/// software. See: http://web.cs.ucdavis.edu/~rogaway/ocb/license.htm
///
/// Based on https://github.com/mumble-voip/mumble/blob/e31d267a11b4ed0597ad41309a7f6b715837141f/src/CryptState.cpp
#[derive(Debug)]
pub struct CryptState {
    aes: Aes128,
    key: [u8; KEY_SIZE],
    // internally as native endianness, externally as little endian and during ocb_* as big endian
    encrypt_nonce: u128,
    decrypt_nonce: u128,
    decrypt_history: [u8; 0x100],

    good: u32,
    late: u32,
    lost: u32,
}

/// The reason a decrypt operation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecryptError {
    /// The packet is too short to be decrypted
    Eof,
    /// The packet has already been decrypted previously.
    Repeat,
    /// The packet was far too late.
    Late,
    /// The MAC of the decrypted packet did not match.
    ///
    /// This may also indicate a substantial de-sync of the decryption nonce.
    Mac,
}

impl DecryptError {
    pub fn to_string(&self) -> &'static str {
        match self {
            DecryptError::Eof => "EOF",
            DecryptError::Repeat => "REPEAT",
            DecryptError::Late => "LATE",
            DecryptError::Mac => "MAC",
        }
    }
}

impl CryptState {
    /// Creates a new CryptState with randomly generated key and initial encrypt- and decrypt-nonce.
    pub fn generate_new() -> Self {
        let mut key = [0; KEY_SIZE];
        fill(&mut key).unwrap();

        CryptState {
            aes: Aes128::new(GenericArray::from_slice(&key)),
            key,
            encrypt_nonce: 0,
            decrypt_nonce: 1 << 127,
            decrypt_history: [0; 0x100],

            good: 0,
            late: 0,
            lost: 0,
        }
    }

    /// Creates a new CryptState from previously generated key, encrypt- and decrypt-nonce.
    pub fn new_from_key(key: [u8; KEY_SIZE]) -> Self {
        CryptState {
            aes: Aes128::new(GenericArray::from_slice(&key)),
            key,
            encrypt_nonce: 0,
            decrypt_nonce: 1 << 127,
            decrypt_history: [0; 0x100],

            good: 0,
            late: 0,
            lost: 0,
        }
    }

    /// Creates a new CryptState from previously generated key, encrypt- and decrypt-nonce.
    pub fn new_from(
        key: [u8; KEY_SIZE],
        encrypt_nonce: [u8; BLOCK_SIZE],
        decrypt_nonce: [u8; BLOCK_SIZE],
    ) -> Self {
        CryptState {
            aes: Aes128::new(GenericArray::from_slice(&key)),
            key,
            encrypt_nonce: u128::from_le_bytes(encrypt_nonce),
            decrypt_nonce: u128::from_le_bytes(decrypt_nonce),
            decrypt_history: [0; 0x100],

            good: 0,
            late: 0,
            lost: 0,
        }
    }

    /// Returns the amount of packets transmitted without issues.
    pub fn get_good(&self) -> u32 {
        self.good
    }

    /// Returns the amount of packets which were transmitted successfully but arrived late.
    pub fn get_late(&self) -> u32 {
        self.late
    }

    /// Returns the amount of packets which were lost.
    pub fn get_lost(&self) -> u32 {
        self.lost
    }

    /// Returns the shared, **private** key.
    pub fn get_key(&self) -> &[u8; KEY_SIZE] {
        &self.key
    }

    /// Returns the nonce used for encrypting.
    pub fn get_encrypt_nonce(&self) -> [u8; BLOCK_SIZE] {
        self.encrypt_nonce.to_le_bytes()
    }

    /// Returns the nonce used for decrypting.
    pub fn get_decrypt_nonce(&self) -> [u8; BLOCK_SIZE] {
        self.decrypt_nonce.to_le_bytes()
    }

    /// Updates the nonce used for decrypting.
    pub fn set_decrypt_nonce(&mut self, nonce: &[u8; BLOCK_SIZE]) {
        self.decrypt_nonce = u128::from_le_bytes(*nonce);
    }

    /// Encrypts an encoded voice packet and returns the resulting bytes.
    pub fn encrypt(&mut self, dst: &mut BytesMut) {
        self.encrypt_nonce = self.encrypt_nonce.wrapping_add(1);

        // Leave four bytes for header
        // dst.resize(4, 0);
        let mut inner = dst.split_off(4);

        // self.codec
        //     .encode(packet, &mut dst[4..])
        //     .expect("VoiceEncoder is infallible");

        let tag = self.ocb_encrypt(inner.as_mut());
        dst.unsplit(inner);

        dst[0] = self.encrypt_nonce as u8;
        dst[1..4].copy_from_slice(&tag.to_be_bytes()[0..3]);
    }

    /// Decrypts a voice packet and (if successful) returns the `Result` of parsing the packet.
    pub fn decrypt(&mut self, buf: &mut BytesMut) -> Result<Result<(), io::Error>, DecryptError> {
        if buf.len() < 4 {
            return Err(DecryptError::Eof);
        }
        let header = buf.split_to(4);
        let nonce_0 = header[0];

        // If we update our decrypt_nonce and the tag check fails or we've been processing late
        // packets, we need to revert it
        let saved_nonce = self.decrypt_nonce;
        let mut late = false; // will always restore nonce if this is the case
        let mut lost = 0; // for stats only

        if self.decrypt_nonce.wrapping_add(1) as u8 == nonce_0 {
            // in order
            self.decrypt_nonce = self.decrypt_nonce.wrapping_add(1);
        } else {
            // packet is late or repeated, or we lost a few packets in between
            let diff = nonce_0.wrapping_sub(self.decrypt_nonce as u8) as i8;
            self.decrypt_nonce = self.decrypt_nonce.wrapping_add(diff as u128);
            if diff > 0 {
                lost = i32::from(diff - 1); // lost a few packets in between this and the last one
            } else if diff > -30 {
                if self.decrypt_history[nonce_0 as usize] == (self.decrypt_nonce >> 8) as u8 {
                    self.decrypt_nonce = saved_nonce;
                    return Err(DecryptError::Repeat);
                }
                // just late
                late = true;
                lost = -1;
            } else {
                return Err(DecryptError::Late); // late by more than 30 packets
            }
        }

        let tag = self.ocb_decrypt(buf.as_mut());
        if verify_slices_are_equal(&tag.to_be_bytes()[0..3], &header[1..4]).is_err() {
            self.decrypt_nonce = saved_nonce;
            return Err(DecryptError::Mac);
        }

        self.decrypt_history[nonce_0 as usize] = (self.decrypt_nonce >> 8) as u8;

        self.good += 1;
        if late {
            self.late += 1;
            self.decrypt_nonce = saved_nonce;
        }
        self.lost = (self.lost as i32 + lost) as u32;

        Ok(Ok(()))
        // Ok(self
        //     .codec
        //     .decode(buf)
        //     .map(|it| it.expect("VoiceCodec is stateless")))
    }

    /// Encrypt the provided buffer using AES-OCB, returning the tag.
    fn ocb_encrypt(&self, mut buf: &mut [u8]) -> u128 {
        let mut offset = self.aes_encrypt(self.encrypt_nonce.to_be());
        let mut checksum = 0u128;

        while buf.len() > BLOCK_SIZE {
            let (chunk, remainder) = buf.split_at_mut(BLOCK_SIZE);
            buf = remainder;
            let chunk: &mut [u8; BLOCK_SIZE] = chunk.try_into().expect("split_at works");

            offset = s2(offset);

            let plain = u128::from_be_bytes(*chunk);
            let encrypted = self.aes_encrypt(offset ^ plain) ^ offset;
            chunk.copy_from_slice(&encrypted.to_be_bytes());

            checksum ^= plain;
        }

        offset = s2(offset);

        let len = buf.len();
        assert!(len <= BLOCK_SIZE);
        let pad = self.aes_encrypt((len * 8) as u128 ^ offset);
        let mut block = pad.to_be_bytes();
        block[..len].copy_from_slice(buf);
        let plain = u128::from_be_bytes(block);
        let encrypted = pad ^ plain;
        buf.copy_from_slice(&encrypted.to_be_bytes()[..len]);

        checksum ^= plain;

        self.aes_encrypt(offset ^ s2(offset) ^ checksum)
    }

    /// Decrypt the provided buffer using AES-OCB, returning the tag.
    /// **Make sure to verify that the tag matches!**
    fn ocb_decrypt(&self, mut buf: &mut [u8]) -> u128 {
        let mut offset = self.aes_encrypt(self.decrypt_nonce.to_be());
        let mut checksum = 0u128;

        while buf.len() > BLOCK_SIZE {
            let (chunk, remainder) = buf.split_at_mut(BLOCK_SIZE);
            buf = remainder;
            let chunk: &mut [u8; BLOCK_SIZE] = chunk.try_into().expect("split_at works");

            offset = s2(offset);

            let encrypted = u128::from_be_bytes(*chunk);
            let plain = self.aes_decrypt(offset ^ encrypted) ^ offset;
            chunk.copy_from_slice(&plain.to_be_bytes());

            checksum ^= plain;
        }

        offset = s2(offset);

        let len = buf.len();
        assert!(len <= BLOCK_SIZE);
        let pad = self.aes_encrypt((len * 8) as u128 ^ offset);
        let mut block = [0; BLOCK_SIZE];
        block[..len].copy_from_slice(buf);
        let plain = u128::from_be_bytes(block) ^ pad;
        buf.copy_from_slice(&plain.to_be_bytes()[..len]);

        checksum ^= plain;

        self.aes_encrypt(offset ^ s2(offset) ^ checksum)
    }

    /// AES-128 encryption primitive.
    fn aes_encrypt(&self, data: u128) -> u128 {
        let mut data_bytes = data.to_be_bytes();
        let block = GenericArray::from_mut_slice(&mut data_bytes);
        self.aes.encrypt_block(block);

        u128::from_be_bytes(data_bytes)
    }

    /// AES-128 decryption primitive.
    fn aes_decrypt(&self, data: u128) -> u128 {
        let mut data_bytes = data.to_be_bytes();
        let block = GenericArray::from_mut_slice(&mut data_bytes);
        self.aes.decrypt_block(block);

        u128::from_be_bytes(data_bytes)
    }
}

fn s2(block: u128) -> u128 {
    let rot = block.rotate_left(1);
    let carry = rot & 1;
    rot ^ (carry * 0x86)
}

impl VoiceCrypter for CryptState {
    fn encrypt(&mut self, buf: &mut BytesMut) {
        self.encrypt(buf)
    }

    fn decrypt(&mut self, buf: &mut BytesMut) -> Result<(), io::Error> {
        self.decrypt(buf)
            .map_err(|decrypt_err| match decrypt_err {
                DecryptError::Eof => io::Error::from(io::ErrorKind::UnexpectedEof),
                _ => io::Error::other(decrypt_err.to_string()),
            })
            .and_then(|inner| inner)
    }

    fn crypt_setup(&self) -> MumbleCryptSetup {
        MumbleCryptSetup {
            key: self.get_key().to_vec(),
            client_nonce: self.get_decrypt_nonce().to_vec(),
            server_nonce: self.get_encrypt_nonce().to_vec(),
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::BufMut;

    use super::*;

    fn u128hex(src: &str) -> u128 {
        u128::from_str_radix(src, 16).unwrap()
    }

    fn bytes_from_hex(src: &str) -> BytesMut {
        let mut buf = BytesMut::new();
        hex_to_bytes(src, &mut buf);
        buf
    }

    fn hex_to_bytes(src: &str, dst: &mut BytesMut) {
        dst.clear();
        dst.reserve(src.len() / 2);
        let mut iter = src.chars();
        while !iter.as_str().is_empty() {
            dst.put_u8(u8::from_str_radix(&iter.as_str()[..2], 16).unwrap());
            iter.next();
            iter.next();
        }
    }

    #[test]
    fn aes_test_vectors() {
        let key = u128hex("E8E9EAEBEDEEEFF0F2F3F4F5F7F8F9FA");
        let state = CryptState::new_from(key.to_be_bytes(), Default::default(), Default::default());
        assert_eq!(
            u128hex("6743C3D1519AB4F2CD9A78AB09A511BD"),
            state.aes_encrypt(u128hex("014BAF2278A69D331D5180103643E99A"))
        );
        assert_eq!(
            u128hex("014BAF2278A69D331D5180103643E99A"),
            state.aes_decrypt(u128hex("6743C3D1519AB4F2CD9A78AB09A511BD"))
        );
    }

    // Test vectors from http://web.cs.ucdavis.edu/~rogaway/papers/draft-krovetz-ocb-00.txt
    // (excluding ones with headers since those aren't implemented here)
    #[test]
    #[allow(clippy::cognitive_complexity)] // all macro-generated
    fn ocb_test_vectors() {
        macro_rules! test_cases {
            ($(
                T : $name:expr,
                M : $plain:expr,
                C : $cipher:expr,
                T : $tag:expr,
            )*) => {$(
                let key = u128hex("000102030405060708090a0b0c0d0e0f");
                let nonce = u128hex("000102030405060708090a0b0c0d0e0f");
                let state = CryptState::new_from(
                    key.to_be_bytes(),
                    nonce.to_be_bytes(),
                    nonce.to_be_bytes(),
                );

                let mut result = BytesMut::new();
                hex_to_bytes($plain.as_ref(), &mut result);
                let tag = state.ocb_encrypt(&mut result);
                assert_eq!(bytes_from_hex($cipher), result, concat!("ENCRYPT-RESULT-", $name));
                assert_eq!(u128hex($tag), tag, concat!("ENCRYPT-TAG-", $name));

                hex_to_bytes($cipher.as_ref(), &mut result);
                let tag = state.ocb_decrypt(&mut result);
                assert_eq!(bytes_from_hex($plain), result, concat!("DECRYPT-RESULT-", $name));
                assert_eq!(u128hex($tag), tag, concat!("DECRYPT-TAG-", $name));
            )*};
        }

        test_cases! {
            T : "OCB-AES-128-0B",
            M : "",
            C : "",
            T : "BF3108130773AD5EC70EC69E7875A7B0",

            T : "OCB-AES-128-8B",
            M : "0001020304050607",
            C : "C636B3A868F429BB",
            T : "A45F5FDEA5C088D1D7C8BE37CABC8C5C",

            T : "OCB-AES-128-16B",
            M : "000102030405060708090A0B0C0D0E0F",
            C : "52E48F5D19FE2D9869F0C4A4B3D2BE57",
            T : "F7EE49AE7AA5B5E6645DB6B3966136F9",

            T : "OCB-AES-128-24B",
            M : "000102030405060708090A0B0C0D0E0F1011121314151617",
            C : "F75D6BC8B4DC8D66B836A2B08B32A636CC579E145D323BEB",
            T : "A1A50F822819D6E0A216784AC24AC84C",

            T : "OCB-AES-128-32B",
            M : "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F",
            C : "F75D6BC8B4DC8D66B836A2B08B32A636CEC3C555037571709DA25E1BB0421A27",
            T : "09CA6C73F0B5C6C5FD587122D75F2AA3",

            T : "OCB-AES-128-40B",
            M : "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F2021222324252627",
            C : "F75D6BC8B4DC8D66B836A2B08B32A6369F1CD3C5228D79FD6C267F5F6AA7B231C7DFB9D59951AE9C",
            T : "9DB0CDF880F73E3E10D4EB3217766688",
        }
    }

    #[test]
    fn encrypt_and_decrypt_are_inverse() {
        let mut server_state =
            CryptState::new_from(Default::default(), Default::default(), Default::default());
        let mut client_state =
            CryptState::new_from(Default::default(), Default::default(), Default::default());

        // Leave 4 byte space
        let packet: Vec<u8> = vec![0, 0, 0, 0, 0, 1, 4, 255, 0, 6];

        let mut buf = BytesMut::from(&packet[..]);
        server_state.encrypt(&mut buf);
        let result = client_state
            .decrypt(&mut buf)
            .expect("Failed to decrypt")
            .expect("Failed to decode");

        assert_eq!(packet[4..], buf);
    }
}
