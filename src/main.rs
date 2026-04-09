use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::{env, process};

mod client;
mod iosered;
mod models;
mod server;
use crate::client::comm_server;
use crate::models::{Client, Mode, PORT};
use crate::server::{comm_client, init_client};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: secmon <server|client> <bind_ip|connect_ip>");
        process::exit(1);
    }

    let mode = match args.get(1).unwrap().as_str() {
        "server" => Mode::Server,
        "client" => Mode::Client,
        _ => {
            eprintln!("Invalid mode; Must be either 'server' or 'client'.");
            process::exit(1);
        }
    };

    let ip_str = args.get(2).unwrap().as_str();
    let ip = match ip_str.parse::<IpAddr>() {
        Ok(ip) => ip,
        _ => {
            eprintln!("Invalid IP address.");
            process::exit(1);
        }
    };

    if mode == Mode::Server {
        let listener = TcpListener::bind((ip, PORT)).unwrap();
        println!("Server listening on {ip}:{PORT}");

        // mutex = (counter: u32, clients: Vec(Client))
        // 'counter' helps find the entry in the vector for the client
        let mutex = Arc::new(Mutex::new((0, Vec::<Client>::new())));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mutex = Arc::clone(&mutex);

                    let serial;
                    {
                        let (counter, clients) = &mut *mutex.lock().unwrap();
                        serial = init_client(&stream, counter, clients);
                    }

                    thread::spawn(move || {
                        if let Err(e) = comm_client(stream, serial, mutex) {
                            eprintln!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("TcpStream error: {}", e);
                }
            };
        }
    } else if mode == Mode::Client {
        let stream = TcpStream::connect((ip, PORT)).unwrap();
        println!("Connected to server {ip}:{PORT}");

        if let Err(e) = comm_server(stream) {
            eprintln!("Connection error: {}", e);
        }
    }
}
