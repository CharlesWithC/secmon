use anyhow::Result;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::UNIX_EPOCH;

use crate::iosered::IOSerialized;
use crate::models::{Client, ClientState, Command, ErrUpdateClient, Response};

mod response;
use crate::server::response::handle_response;

/// Initialize client connection in `clients` list.
///
/// Returns `serial` of the client for later identification.
fn handle_new_client(address: &SocketAddr, hostname: &String, client_state: &ClientState) -> u32 {
    let mut guard = client_state.lock().unwrap();
    let (ref mut counter, ref mut clients) = *guard;

    // increment counter for client
    *counter += 1;

    // initialize client, add client to vector
    let client = Client {
        serial: *counter,
        address: address.clone(),
        hostname: hostname.clone(),
        sessions: Err("Initializing".to_owned()),
        wg_peers: Err("Initializing".to_owned()),
        last_update: UNIX_EPOCH,
    };
    clients.push(client);

    // return client serial
    *counter
}

/// Main thread function for each client connection
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, client_state: ClientState) -> Result<()> {
    let mut serial;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        serial = handle_new_client(&address, &hostname, &client_state);
        println!("{address} connected as {hostname} and was assigned serial {serial}");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    let command = Command::ReportSyncStart;
    println!("Sending {command} to {address}");
    stream.write(&command)?;

    loop {
        let response = stream.read::<Response>()?;
        println!("{address} responded {response}");

        let result = handle_response(serial, response, &client_state);
        if let Err(error) = result {
            match error {
                ErrUpdateClient::SerialNotRecognized => {
                    eprintln!("{address} is not a recognized client");
                    serial = handle_new_client(&address, &hostname, &client_state);
                }
            }
        }
    }
}

/// Server-side main function to communicate with client.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: TcpListener) -> Result<()> {
    // mutex = (counter: u32, clients: Vec(Client))
    // 'counter' helps find the entry in the vector for the client
    let client_state: ClientState = Arc::new(Mutex::new((0, Vec::<Client>::new())));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let client_state = Arc::clone(&client_state);

                thread::spawn(move || {
                    if let Err(e) = thread_main(stream, client_state) {
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
