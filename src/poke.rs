extern crate crypto;
extern crate byteorder;
extern crate rand;
extern crate mio;
extern crate bytes;

mod crypto_stream;

use crypto_stream::{CryptoStream, CryptoStruct};

fn main() {
    let mut buf = Vec::<u8>::new();

    let mut s1 = CryptoStruct::new(&mut |data| {
        buf.extend(data);
    });

    let mut s2 = CryptoStruct::new(&mut |data| {
        s1.decrypt(data, &mut |data| {
            panic!("This is weird!");
        });
    });

    s2.decrypt(&buf[..], &mut |data| {
        panic!("This is weird2!");
    });

    s1.encrypt(b"lol, wtf", &mut |data| {
        s2.decrypt(data, &mut |data| {
            println!("What is this {:?}?", data);
        });
    });

    s2.encrypt(b"lol, wtf", &mut |data| {
        s1.decrypt(data, &mut |data| {
            println!("What is this {:?}?", data);
        });
    });

    buf.truncate(0);

    s1.encrypt(b"lol, monkey1", &mut |data| {
        buf.extend(data);
    });

    s1.encrypt(b"lol, monkey2", &mut |data| {
        buf.extend(data);
    });

    s1.encrypt(b"lol, monkey3", &mut |data| {
        buf.extend(data);
    });

    s2.decrypt(&buf[..], &mut |data| {
        println!("Your data: {:?}", data);
    });
}
