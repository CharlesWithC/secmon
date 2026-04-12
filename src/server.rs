use std::io::Result;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::iosered::IOSerialized;
use crate::models::{Client, Command, Message};

pub type ClientState = Arc<Mutex<(u32, Vec<Client>)>>;

/// Initialize client connection in `clients` list.
///
/// Increments `counter` and set 'serial' to the incremented `counter`.
///
/// Returns `serial` of the client for later identification.
pub fn init_client(stream: &TcpStream, counter: &mut u32, clients: &mut Vec<Client>) -> u32 {
    // increment counter for client
    *counter += 1;

    // initialize client, add client to vector
    let client = Client {
        serial: *counter,
        address: stream.peer_addr().unwrap(),
        sessions: Vec::new(),
        wg_peers: Vec::new(),
        last_update: SystemTime::now(),
    };
    println!("Client connected: {}", client.address);
    clients.push(client);

    // return client serial
    *counter
}

/// Server-side main function to communicate with client identified by `serial`.
///
/// `init_client` must be called before calling this function.
///
/// This is a blocking function and does not exit until connection is closed.
pub fn comm_client(mut stream: TcpStream, mut serial: u32, mutex: ClientState) -> Result<()> {
    let address = stream.peer_addr().unwrap();
    loop {
        let command = Command::Report;

        println!("Sending {} to {address}", command);
        stream.write(&command)?;

        let message = stream.read::<Message>()?;
        let (counter, clients) = &mut *mutex.lock().unwrap();
        if let Some(index) = clients.iter().position(|client| client.serial == serial) {
            println!("{address} responded {message}");
            match message {
                Message::Report(sessions, wg_peers) => {
                    for session in sessions.iter() {
                        println!("{session}");
                    }
                    for wg_peer in wg_peers.iter() {
                        println!("{wg_peer}");
                    }
                    clients[index].sessions = sessions;
                    clients[index].wg_peers = wg_peers;
                    clients[index].last_update = SystemTime::now();
                }
                _ => {}
            }
        } else {
            eprintln!("{address} is no longer a recognized client");
            serial = init_client(&stream, counter, clients);
        }

        thread::sleep(Duration::from_secs(5));
    }
}
