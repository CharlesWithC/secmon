use std::net::{IpAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::{env, process};

mod client;
mod iosered;
mod models;
mod server;
use crate::models::{DEFAULT_HOST, DEFAULT_PORT, Mode};

fn get_env_var<T: FromStr + ToString>(key: &str, default: Option<T>) -> T
where
    T::Err: std::fmt::Debug,
{
    env::var(key)
        .unwrap_or_else(|_| {
            if let Some(value) = default {
                return value.to_string();
            } else {
                eprintln!("Missing env var: {key}");
                process::exit(1);
            }
        })
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse {key}: {:?}", e);
            process::exit(1);
        })
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: secmon <server|client>");
        process::exit(1);
    }

    let ip: IpAddr;
    let port: u16;
    let mode = match args.get(1).unwrap().as_str() {
        "server" => {
            ip = get_env_var("HOST", Some(DEFAULT_HOST));
            port = get_env_var("PORT", Some(DEFAULT_PORT));
            Mode::Server
        }
        "client" => {
            ip = get_env_var("SERVER_IP", None);
            port = get_env_var("SERVER_PORT", Some(DEFAULT_PORT));
            Mode::Client
        }
        _ => {
            eprintln!("Invalid mode; Must be either 'server' or 'client'");
            process::exit(1);
        }
    };

    if mode == Mode::Server {
        let listener = TcpListener::bind((ip, port)).unwrap();
        println!("Server listening on {ip}:{port}");

        if let Err(e) = crate::server::main(listener) {
            eprintln!("Connection error: {}", e);
        }
    } else if mode == Mode::Client {
        let stream = TcpStream::connect((ip, port)).unwrap();
        println!("Connected to server {ip}:{port}");

        if let Err(e) = crate::client::main(stream) {
            eprintln!("Connection error: {}", e);
        }
    }
}
