use crypto::aead::{AeadEncryptor,AeadDecryptor};
use crypto::chacha20poly1305::ChaCha20Poly1305;
use crypto::curve25519::{curve25519_base, curve25519};
use crypto::blake2b::Blake2b;
use crypto::digest::Digest;
use byteorder::{LittleEndian,WriteBytesExt,ReadBytesExt,Error};
use std::io::{Read,Write};
use std::io;
use rand::os::OsRng;
use rand::Rng;
use bytes::{MutBuf,RingBuf};
use byteorder;
use std::mem::size_of;
use std::result;

const TAG_SIZE: usize = 16;
const NONCE_SIZE: usize = 16;
const BOXKEY_SIZE: usize = 16;
const PUBLIC_KEY_SIZE: usize = 32;
const SECRET_KEY_SIZE: usize = 32;

pub trait CryptoStream {
    fn new<F>(send_to_world: &mut F) -> Self
        where F : FnMut(&[u8]);

    fn encrypt<F>(&mut self, data: &[u8], send_to_world: &mut F)
        where F : FnMut(&[u8]);

    fn decrypt<F>(&mut self, data: &[u8], recv_to_local_process: &mut F)
        where F : FnMut(&[u8]);
}

struct BoxState {
    nonce: [u8; NONCE_SIZE],
    key: [u8; BOXKEY_SIZE],
    pos: u64
}

impl BoxState {
    fn new(seed_key: &[u8; BOXKEY_SIZE]) -> BoxState {
        let mut nonce = [0; NONCE_SIZE];
        let mut key = [0; BOXKEY_SIZE];

        Blake2b::blake2b(&mut nonce, seed_key, b"nonce");
        Blake2b::blake2b(&mut key, seed_key, b"key");
        BoxState {
            nonce: nonce,
            key: key,
            pos: 0
        }
    }

    fn increment(&mut self) {
        for i in 0..self.nonce.len() {
            self.nonce[i] += 1;
            if self.nonce[i] != 0 {
                break;
            }
        }
        self.pos += 1;
    }

    fn make_state(&mut self) -> ChaCha20Poly1305 {
        /* Create the additional data */
        let mut additional_data = [0; 8];
        (&mut additional_data[0..]).write_u64::<LittleEndian>(self.pos);

        let res =
            ChaCha20Poly1305::new(&self.key,
                                  &self.nonce,
                                  &additional_data);
        self.increment();

        return res;
    }

    fn encrypt(&mut self, input: &[u8], output: &mut [u8], tag: &mut [u8]) {
        self.make_state().encrypt(input, output, tag);
    }

    fn decrypt(&mut self, input: &[u8], output: &mut [u8], tag: &[u8])
               -> bool {
        self.make_state().decrypt(input, output, tag)
    }
}

type Encrypter = BoxState;
type Decrypter = BoxState;

pub struct CryptoStruct {
    state: CryptoState,
    buf: RingBuf,
}

pub enum CryptoState {
    PreHandshake([u8; SECRET_KEY_SIZE], [u8; PUBLIC_KEY_SIZE]),
    Ready(Encrypter, Decrypter)
}
use self::CryptoState::{PreHandshake,Ready};

impl CryptoStream for CryptoStruct {
    fn new<F>(send_to_world: &mut F) -> CryptoStruct
        where F : FnMut(&[u8]) {
        let mut rng = OsRng::new().expect("Could not get random number generator");
        let mut secret_key = [0; SECRET_KEY_SIZE];
        rng.fill_bytes(&mut secret_key);
        let public_key = curve25519_base(&secret_key);
        send_to_world(&public_key);
        CryptoStruct {
            state: PreHandshake(secret_key, public_key),
            buf: RingBuf::new(0x10000 + 2 + TAG_SIZE)
        }
    }

    fn encrypt<F>(&mut self, data: &[u8], send_to_world: &mut F)
        where F : FnMut(&[u8]) {
        if let &mut Ready(ref mut encrypter, ref mut decrypter) = &mut self.state {
            assert!(data.len() <= 0x10000);
            let mut outbuf = [0; 0x10000 + TAG_SIZE];
            let mut outbuf = &mut outbuf[0..data.len() + TAG_SIZE];
            {
                let (outmsg, tag) = outbuf.split_at_mut(data.len());
                encrypter.encrypt(data, outmsg, tag);
            }
            send_to_world(outbuf);
        } else {
            panic!("encrypt should not be called before handshake is done.");
        }
    }

    fn decrypt<F>(&mut self, data: &[u8], recv_to_local_process: &mut F)
        where F : FnMut(&[u8]) {

        let mut ndx = 0;

        while ndx < data.len() {
            ndx += self.buf.write_slice(&data[ndx..]);

            // Handled packets until error
            loop {
                self.buf.mark();
                let res = if self.handshake_done() {
                    self.handle_packet(recv_to_local_process)
                } else {
                    self.handle_handshake()
                };

                if res.is_err() {
                    self.buf.reset();
                    break;
                }
            }
        }
    }
}

struct EmptyError;
impl From<byteorder::Error> for EmptyError {
    fn from(err: byteorder::Error) -> EmptyError {
        EmptyError
    }
}

impl From<io::Error> for EmptyError {
    fn from(err: io::Error) -> EmptyError {
        EmptyError
    }
}

type Result<T> = result::Result<T, EmptyError>;

impl CryptoStruct {
    fn handshake_done(&self) -> bool {
        match self.state {
            PreHandshake(..) => false,
            Ready(..) => true
        }
    }

    fn handle_handshake(&mut self) -> Result<()> {
        let mut their_public_key = [0; PUBLIC_KEY_SIZE];
        try!(self.buf.read_exact(&mut their_public_key));

        if let PreHandshake(secret_key, our_public_key) = self.state {
            let shared: [u8; PUBLIC_KEY_SIZE] = curve25519(&secret_key, &their_public_key);
            let mut their_seed_key = [0; BOXKEY_SIZE];
            let mut our_seed_key = [0; BOXKEY_SIZE];

            let mut hasher = Blake2b::new(BOXKEY_SIZE);

            hasher.input(&shared);
            hasher.input(&their_public_key);
            hasher.input(&our_public_key);
            hasher.result(&mut their_seed_key);
            hasher.reset();

            hasher.input(&shared);
            hasher.input(&our_public_key);
            hasher.input(&their_public_key);
            hasher.result(&mut our_seed_key);
            hasher.reset();

            self.state = Ready(
                BoxState::new(&our_seed_key),
                BoxState::new(&their_seed_key));

            Ok(())
        } else {
            panic!("handle_handshake should not be called before handshake is done.");
        }
    }

    fn handle_packet<F>(&mut self, recv_to_local_process: &mut F) -> Result<()>
        where F : FnMut(&[u8]) {

        let size = try!(self.buf.read_u16::<LittleEndian>()) as usize;

        let mut inbuf = [0; 0x10000];
        let mut inbuf = &mut inbuf[0..size];
        let mut outbuf = [0; 0x10000];
        let mut outbuf = &mut outbuf[0..size];
        let mut tag = [0; TAG_SIZE];

        try!(self.buf.read_exact(inbuf));
        try!(self.buf.read_exact(&mut tag));

        if let &mut Ready(ref mut encrypter, ref mut decrypter) = &mut self.state {
            assert!(decrypter.decrypt(outbuf, inbuf, &tag) == true);
            recv_to_local_process(outbuf);
        } else {
            panic!("handle_packet should not be called after handshake is done.");
        }

        Ok(())
    }
}
