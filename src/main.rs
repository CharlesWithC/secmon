use anyhow::Result;
use std::net::IpAddr;
use std::str::FromStr;
use std::{env, process};

mod exec;
mod hub;
mod iosered;
mod models;
mod node;
use crate::models::{DEFAULT_HOST, DEFAULT_PORT, Mode, NodeConfig};

const USAGE: &str = "Usage:
  secmon hub                    launch hub server
  secmon node [who] [wg]        launch node server
    [--reconnect]               reconnect if connection lost

Hub control commands:
  secmon list                   list all connected nodes

Environment:
  HOST=<host> PORT=<port> secmon hub
  HUB_IP=<ip> HUB_PORT=<port> secmon node";

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

fn launch(ip: IpAddr, port: u16, mode: Mode) -> Result<()> {
    match &mode {
        &Mode::Hub => {
            crate::hub::main(ip, port)?;
        }
        &Mode::Node(node_config) => loop {
            crate::node::main(ip, port, node_config)?;
        },
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("{USAGE}");
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

            let reconnect = args.contains(&"--reconnect".to_owned());
            let enable_sessions = args.contains(&"who".to_owned());
            let enable_wg_peers = args.contains(&"wg".to_owned());

            Mode::Node(NodeConfig {
                reconnect,
                enable_sessions,
                enable_wg_peers,
            })
        }
        _ => {
            eprintln!("Invalid mode; Must be either 'hub' or 'node'");
            process::exit(1);
        }
    };

    if let Err(err) = launch(ip, port, mode) {
        eprintln!("Error: {}", err);
        process::exit(1);
    } else {
        process::exit(0);
    }
}
