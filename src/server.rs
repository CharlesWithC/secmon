use anyhow::Result;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::iosered::IOSerialized;
use crate::models::{Client, ClientState, Command, ErrUpdateClient, Response};

mod response;
use crate::server::response::handle_response;

/// Initialize client connection in `clients` list.
///
/// Returns `serial` of the client for later identification.
fn handle_new_client(address: &SocketAddr, hostname: &String, mutex: &ClientState) -> u32 {
    let (counter, clients) = &mut *mutex.lock().unwrap();

    // increment counter for client
    *counter += 1;

    // initialize client, add client to vector
    let client = Client {
        serial: *counter,
        address: address.clone(),
        hostname: hostname.clone(),
        sessions: Ok(Vec::new()),
        wg_peers: Ok(Vec::new()),
        last_update: SystemTime::now(),
    };
    clients.push(client);

    // return client serial
    *counter
}

/// Main thread function for each client connection
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, mutex: ClientState) -> Result<()> {
    let mut serial;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        serial = handle_new_client(&address, &hostname, &mutex);
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    println!("{address} connected as {hostname}");

    loop {
        let command = Command::Report;

        println!("Sending {} to {address}", command);
        stream.write(&command)?;

        let response = stream.read::<Response>()?;
        println!("{address} responded {response}");

        let result = handle_response(serial, response, &mutex);
        if let Err(error) = result {
            match error {
                ErrUpdateClient::SerialNotRecognized => {
                    eprintln!("{address} is not a recognized client");
                    serial = handle_new_client(&address, &hostname, &mutex);
                }
            }
        }

        thread::sleep(Duration::from_secs(5));
    }
}

/// Server-side main function to communicate with client.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: TcpListener) -> Result<()> {
    // mutex = (counter: u32, clients: Vec(Client))
    // 'counter' helps find the entry in the vector for the client
    let mutex = Arc::new(Mutex::new((0, Vec::<Client>::new())));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mutex = Arc::clone(&mutex);

                thread::spawn(move || {
                    if let Err(e) = thread_main(stream, mutex) {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("TcpStream error: {}", e);
            }
        };
    }

    Ok(())
}
