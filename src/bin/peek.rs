extern crate mio;
extern crate bytes;

use std::io::Cursor;
use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use mio::util::Slab;
use bytes::{Buf, Take};

const SERVER: mio::Token = mio::Token(0);

// Server state
struct Peek {
    server: TcpListener,
    connections: Slab<Connection>,
}

impl Peek {
    fn new(server: TcpListener) -> Peek {
        let slab = Slab::new_starting_at(mio::Token(1), 1024);

        Peek {
            server: server,
            connections: slab,
        }
    }
}


// Connections
struct Connection {
    socket: TcpStream,
    token: mio::Token,
    state: State,
}

enum State {
    Reading(Vec<u8>),
    Writing(Take<Cursor<Vec<u8>>>),
}

impl Connection {
    fn new(socket: TcpStream, token: mio::Token) -> Connection {
        Connection {
            socket: socket,
            token: token,
            state: State::Reading(vec![]),
        }
    }
    fn ready(&mut self, event_loop: &mut mio::EventLoop<Peek>, events: mio::EventSet) {

    }
}

// Handle connections
impl mio::Handler for Peek {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<Peek>,
             token: mio::Token,
             events: mio::EventSet) {
        match token {
            SERVER => {
                // Only receive readable events
                assert!(events.is_readable());

                println!("the server socket is ready to accept a connection");
                match self.server.accept() {
                    Ok(Some((mut socket, addr))) => {
                        let token = self.connections
                                        .insert_with(|token| Connection::new(socket, token))
                                        .unwrap();

                        event_loop.register(&self.connections[token].socket,
                                            token,
                                            mio::EventSet::readable(),
                                            mio::PollOpt::edge() | mio::PollOpt::oneshot())
                                  .unwrap();
                    }
                    Ok(None) => {
                        println!("the server socket wasn't actually ready");
                    }
                    Err(e) => {
                        println!("listener.accept() errored: {}", e);
                        event_loop.shutdown();
                    }
                }
            }
            _ => {
                self.connections[token].ready(event_loop, events);
            }
        }
    }
}

fn main() {
    // Broadcast our presence
    //let mut socket = try!(UdpSocket::bind("127.0.0.1:34254"));


    // Start event loop
    let address = "0.0.0.0:1337".parse().unwrap();
    let server = TcpListener::bind(&address).unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER, EventSet::readable(), PollOpt::edge()).unwrap();

    println!("Peek started!");
    let mut peek = Peek::new(server);
    event_loop.run(&mut peek).unwrap();
}
