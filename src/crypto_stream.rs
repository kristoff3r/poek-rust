use crypto::aead::{AeadEncryptor,AeadDecryptor};
use crypto::chacha20poly1305::ChaCha20Poly1305;
use crypto::curve25519::{curve25519_base, curve25519};
use byteorder::{LittleEndian,WriteBytesExt};
use std::io::Write;
use rand::os::OsRng;
use rand::Rng;
use bytes::RingBuf;

pub trait CryptoStream {
    fn new<F>(send_to_world: F) -> Self
        where F : FnMut(&[u8]);

    fn encrypt<F>(&mut self, data: &[u8], send_to_world: F)
        where F : FnMut(&[u8]);

    fn decrypt<F>(&mut self, data: &[u8], recv_to_local_process: F)
        where F : FnMut(&[u8]);
}

struct BoxState {
    nonce: [u8; 16],
    key: [u8; 32],
    pos: u64
}

impl BoxState {
    fn increment(&mut self) {
        for i in 0..16 {
            self.nonce[i] += 1;
            if self.nonce[i] != 0 {
                break;
            }
        }
        self.pos += 1;
    }

    fn make_state(&mut self, size: usize) -> ChaCha20Poly1305 {
        /* Create the additional data */
        let mut additional_data = [0; 8+8];
        (&mut additional_data[0..8]).write_u64::<LittleEndian>(self.pos);
        (&mut additional_data[8..16]).write_u64::<LittleEndian>(size as u64);
        let additional_data = additional_data;

        let res =
            ChaCha20Poly1305::new(&self.key,
                                  &self.nonce,
                                  &additional_data);
        self.increment();

        return res;
    }

    fn encrypt(&mut self, input: &[u8], output: &mut [u8], tag: &mut [u8]) {
        self.make_state(input.len()).encrypt(input, output, tag);
    }

    fn decrypt(&mut self, input: &[u8], output: &mut [u8], tag: &mut [u8]) {
        self.make_state(input.len()).encrypt(input, output, tag);
    }
}

type Encrypter = BoxState;
type Decrypter = BoxState;

pub struct CryptoStruct {
    state: CryptoState,
    buf: RingBuf
}

pub enum CryptoState {
    PreHandshake([u8; 32]),
    Ready(Encrypter, Decrypter)
}

impl CryptoStream for CryptoStruct {
    fn new<F>(mut send_to_world: F) -> CryptoStruct
        where F : FnMut(&[u8]) {
        let mut rng = OsRng::new().expect("Could not get random number generator");
        let mut key = [0; 32];
        rng.fill_bytes(&mut key);
        send_to_world(&curve25519_base(&key));
        CryptoStruct {
            state: CryptoState::PreHandshake(key),
            buf: RingBuf::new(65538)
        }
    }

    fn encrypt<F>(&mut self, data: &[u8], send_to_world: F)
        where F : FnMut(&[u8]) {
    }

    fn decrypt<F>(&mut self, data: &[u8], recv_to_local_process: F)
        where F : FnMut(&[u8]) {

    }
}
