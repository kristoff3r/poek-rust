//! An echo server that times out
//!
//! The server can be run by executing:
//!
//! ```
//! cargo run --example echo_server_with_timeout
//! ```
//!
//! Then connect to it using telnet.

extern crate futures;
extern crate tokio_core as tokio;
extern crate tokio_line as line;
extern crate tokio_service as service;
extern crate tokio_middleware as middleware;
extern crate env_logger;

use futures::Future;
use tokio::reactor::Core;
use std::io;

pub fn main() {
    env_logger::init().unwrap();

    let mut lp = Core::new().unwrap();

    // The address to bind the listener socket to
    let addr = "127.0.0.1:12345".parse().unwrap();

    // The service to run
    let service = {
        service::simple_service(move |msg| {
            Ok(msg)
        })
    };

    // Decorate the service with the Log middleware
    let service = middleware::Log::new(service);

    // Start the server
    line::service::serve(&lp.handle(), addr, service).unwrap();

    println!("Echo server running on {}", addr);

    lp.run(futures::empty::<(), ()>()).unwrap();
}
