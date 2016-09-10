use bytes::{self, Buf, BlockBuf, MutBuf};
use std::{io, str};
use std::fmt::Write;
use tokio::io::Io;
use proto::{pipeline, Parse, Serialize, Framed};
use low_level_transport::Frame;

pub struct Parser;

impl Parse for Parser {
    type Out = Frame;

    fn parse(&mut self, buf: &mut BlockBuf) -> Option<Frame> {
        // Make sure the data is continuous in memory. BlockBuf is 'faking' a continuous buffer -
        // if you receive two TCP packets, block buf will keep two allocated memory blocks around -
        // this is very efficient for reading, but since we call the 'bytes' method below which
        // requires a single continous block of memory, we need to ask blockbuf to defrag itself. 
        if !buf.is_compact() {
            buf.compact();
        }

        // If our buffer contains a newline...
        if buf.len() >= 100 {
            // remove this line and the newline from the buffer.
            let n = 100;
            let buf_ = buf.shift(n);

            // Turn this data into a UTF string and return it in a Frame.
            return Some(pipeline::Frame::Message(buf_.buf().bytes().into()));
        }
        None
    }
}

pub struct Serializer;

impl Serialize for Serializer {
    type In = Frame;

    fn serialize(&mut self, frame: Frame, buf: &mut BlockBuf) {
        use proto::pipeline::Frame::*;

        match frame {
            Message(vec) => {
                buf.write_slice(&vec);
            }
            Error(e) => {
                let _ = write!(bytes::Fmt(buf), "[ERROR] {}\n", e);
            }
            MessageWithBody(..) | Body(..) => {
                // Our Line protocol does not support streaming bodies
                unreachable!();
            }
            Done => {}
        }
    }
}

pub type FramedLineTransport<T> = Framed<T, Parser, Serializer>;

pub fn new_line_transport<T>(inner: T) -> FramedLineTransport<T>
    where T: Io,
{
  Framed::new(inner,
              Parser,
              Serializer,
              BlockBuf::default(),
              BlockBuf::default())
}
