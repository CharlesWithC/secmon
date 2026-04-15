use anyhow::Result;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::iosered::IOSerialized;
use crate::models::{Client, Command, Response};

type ClientState = Arc<Mutex<(u32, Vec<Client>)>>;

/// Error on updating client information
enum ErrUpd {
    /// Client cannot be recognized based on serial
    NotRecognized,
}

/// Initialize client connection in `clients` list.
///
/// Increments `counter` and set 'serial' to the incremented `counter`.
///
/// Returns `serial` of the client for later identification.
fn init_client(stream: &TcpStream, mutex: &ClientState, hostname: &String) -> u32 {
    let (counter, clients) = &mut *mutex.lock().unwrap();

    // increment counter for client
    *counter += 1;

    // initialize client, add client to vector
    let client = Client {
        serial: *counter,
        hostname: hostname.clone(),
        address: stream.peer_addr().unwrap(),
        sessions: Ok(Vec::new()),
        wg_peers: Ok(Vec::new()),
        last_update: SystemTime::now(),
    };
    clients.push(client);

    // return client serial
    *counter
}

/// Process 'response' and update client information for client with given serial
fn updt_client(serial: u32, mutex: &ClientState, response: Response) -> Result<(), ErrUpd> {
    let (_, clients) = &mut *mutex.lock().unwrap();
    if let Some(index) = clients.iter().position(|client| client.serial == serial) {
        match response {
            Response::Report(sessions, wg_peers) => {
                match &sessions {
                    Ok(sessions) => {
                        for session in sessions.iter() {
                            println!("{session}");
                        }
                    }
                    Err(error) => {
                        println!("Error: {error}");
                    }
                }
                match &wg_peers {
                    Ok(wg_peers) => {
                        for wg_peer in wg_peers.iter() {
                            println!("{wg_peer}");
                        }
                    }
                    Err(error) => {
                        println!("Error: {error}");
                    }
                }

                clients[index].sessions = sessions;
                clients[index].wg_peers = wg_peers;
                clients[index].last_update = SystemTime::now();
            }
            _ => {}
        }

        Ok(())
    } else {
        Err(ErrUpd::NotRecognized)
    }
}

/// Server-side main function to communicate with client.
///
/// This is a blocking function and does not exit until connection is closed.
pub fn comm_client(mut stream: TcpStream, mutex: ClientState) -> Result<()> {
    let mut serial;
    let hostname;
    let address = stream.peer_addr().unwrap();

    if let Response::Connect(host) = stream.read::<Response>()? {
        hostname = host;
        serial = init_client(&stream, &mutex, &hostname);
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

        let result = updt_client(serial, &mutex, response);
        if let Err(error) = result {
            match error {
                ErrUpd::NotRecognized => {
                    eprintln!("{address} is no longer a recognized client");
                    serial = init_client(&stream, &mutex, &hostname);
                }
            }
        }

        thread::sleep(Duration::from_secs(5));
    }
}
