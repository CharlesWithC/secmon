use std::net::{IpAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::{env, process};

mod exec;
mod hub;
mod iosered;
mod models;
mod node;
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
        eprintln!("Usage: secmon <hub|node>");
        process::exit(1);
    }

    let ip: IpAddr;
    let port: u16;
    let mode = match args.get(1).unwrap().as_str() {
        "hub" => {
            ip = get_env_var("HOST", Some(DEFAULT_HOST));
            port = get_env_var("PORT", Some(DEFAULT_PORT));
            Mode::Hub
        }
        "node" => {
            ip = get_env_var("HUB_IP", None);
            port = get_env_var("HUB_PORT", Some(DEFAULT_PORT));
            Mode::Node
        }
        _ => {
            eprintln!("Invalid mode; Must be either 'hub' or 'node'");
            process::exit(1);
        }
    };

    if mode == Mode::Hub {
        let listener = TcpListener::bind((ip, port)).unwrap();
        println!("Hub listening on {ip}:{port}");

        if let Err(e) = crate::hub::main(listener) {
            eprintln!("Connection error: {}", e);
        }
    } else if mode == Mode::Node {
        let stream = TcpStream::connect((ip, port)).unwrap();
        println!("Connected to hub {ip}:{port}");

        if let Err(e) = crate::node::main(stream) {
            eprintln!("Connection error: {}", e);
        }
    }
}
