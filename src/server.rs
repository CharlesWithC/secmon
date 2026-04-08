use std::io::Result;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::comm::SendRecv;
use crate::models::{Client, Command, Message};

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
    println!("Client connected: {}.", client.address);
    clients.push(client);

    // return client serial
    *counter
}

pub fn comm_client(
    mut stream: TcpStream,
    mut serial: u32,
    mutex: Arc<Mutex<(u32, Vec<Client>)>>,
) -> Result<()> {
    loop {
        let command = Command::Report;
        stream.send(&command)?;

        let message = stream.recv::<Message>()?;
        if let Message::Report(sessions, wg_peers) = message {
            let (counter, clients) = &mut *mutex.lock().unwrap();
            if let Some(index) = clients.iter().position(|client| client.serial == serial) {
                clients[index].sessions = sessions;
                clients[index].wg_peers = wg_peers;
                clients[index].last_update = SystemTime::now();
                println!("Received update from {}.", clients[index].address);
            } else {
                eprintln!("Client is no longer in the data list; Client will be reinitialized.");
                serial = init_client(&stream, counter, clients);
            }
        } else {
            eprintln!("Client did not respond with a report; This should not happen.");
        }

        thread::sleep(Duration::from_secs(5));
    }
}
