//! This a basic example how to setup NAT (Network Address Translation).
//!
//! 1. it creates publicly accessible server node.
//! 2. client node is created and put under NAT
//! 3. client connects to the server
//!
//! When you run the example, you can see that client node sees its LAN IP address and when it
//! connects to the server, server sees its public IP - one that NAT device owns.

extern crate future_utils;
extern crate futures;
extern crate netsim;
extern crate tokio;
#[macro_use]
extern crate unwrap;

use futures::future::Future;
use futures::sync::oneshot;
use futures::Stream;
use netsim::device::ipv4::Ipv4NatBuilder;
use netsim::{node, spawn, Ipv4Range, Network};
use std::net::{SocketAddr, SocketAddrV4};
use std::thread;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

fn main() {
    // tokio event loop
    let mut evloop = unwrap!(Runtime::new());
    let network = Network::new();

    let (server_addr_tx, server_addr_rx) = oneshot::channel();
    let server = node::ipv4::machine(move |ip| {
        // This code is run on a separate thread.
        println!("[server] {}, thread: {:?}", ip, thread::current().id());

        let mut evloop = unwrap!(Runtime::new());

        let bind_addr = SocketAddr::V4(SocketAddrV4::new(ip, 0));
        let listener = unwrap!(TcpListener::bind(&bind_addr));
        let _ = server_addr_tx.send(unwrap!(listener.local_addr()));

        let accept_conns = listener.incoming().for_each(|stream| {
            let addr = unwrap!(stream.peer_addr());
            println!("[server] Client connected: {}", addr);
            Ok(())
        });
        let _ = unwrap!(evloop.block_on(accept_conns));
    });
    let client = node::ipv4::machine(move |ip| {
        // This code is run on a separate thread.
        println!("[client] {}, thread: {:?}", ip, thread::current().id());

        let server_addr = unwrap!(server_addr_rx.wait());
        println!("[client] Got server addr: {}", server_addr);
        let mut evloop = unwrap!(Runtime::new());

        let connect = TcpStream::connect(&server_addr).and_then(|_stream| {
            println!("[client] Connected to server");
            Ok(())
        });
        let _ = unwrap!(evloop.block_on(connect));
    });
    let client = node::ipv4::nat(Ipv4NatBuilder::default(), client);

    let router = node::ipv4::router((server, client));
    let (spawn_complete, _ipv4_plug) =
        spawn::ipv4_tree(&network.handle(), Ipv4Range::global(), router);
    unwrap!(evloop.block_on(spawn_complete));
}
