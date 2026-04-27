use anyhow::{Result, anyhow};
use std::fs;
use std::{env, process};

mod hub;
mod models;
mod node;
mod traits;
mod utils;
use crate::models::{DEFAULT_HOST, DEFAULT_PORT, DEFAULT_SOCKET_DIR, LaunchArgs, NodeConfig};
use crate::utils::{get_env_var, get_env_var_strict};

const USAGE: &str = "Usage:
  secmon hub                        launch hub server
  secmon node [who] [wg]            launch node server
    [--reconnect]
  secmon help                       print this help message

Utility commands:
  secmon list [sorted]              list all connected nodes
  secmon subscribe                  subscribe to node state atomic updates
  secmon <node> execute <label>     execute a preconfigured allowed command

  <node> can be address or hostname, or \"-\" for all connected nodes.

Environment:
  hub:      HOST=<host> PORT=<port>
  node:     HUB_IP=<ip> HUB_PORT=<port>
            COMMAND_ALLOWLIST_FILE=<path>

COMMAND_ALLOWLIST_FILE:
  A file containing commands that may be executed by hub.
  Separate label and command with '=', and provide one pair in each line.
  Label must not contain '=', and command must finish in one line.
  Example:
    LABEL=COMMAND
    update=apt update
    reboot=shutdown -r";

fn get_socket_path() -> String {
    let mut socket_path = DEFAULT_SOCKET_DIR.to_owned() + "/secmon.sock";
    if let Some(dir) = get_env_var::<String>("XDG_RUNTIME_DIR", None).unwrap() {
        if !dir.ends_with("/0") {
            // non-root
            socket_path = dir + "/secmon.sock";
        }
    }

    socket_path
}

fn launch(launch_args: LaunchArgs) -> Result<()> {
    match launch_args {
        LaunchArgs::Hub(ip, port) => {
            let socket_path = get_socket_path();
            if fs::exists(&socket_path)
                .map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))?
            {
                fs::remove_file(&socket_path)
                    .map_err(|e| anyhow!("Unable to unlink {socket_path}: {e}"))?;
            }

            crate::hub::main_daemon(ip, port, socket_path)?;
        }
        LaunchArgs::Client(command) => {
            let socket_path = get_socket_path();
            if !fs::exists(&socket_path)
                .map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))?
            {
                return Err(anyhow!(
                    "{socket_path} does not exist, is hub daemon running?"
                ));
            }

            crate::hub::main_client(socket_path, command)?;
        }
        LaunchArgs::Node(ip, port, node_config) => {
            crate::node::main(ip, port, node_config)?;
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{USAGE}");
        process::exit(1);
    }

    let launch_args = match args.get(1).unwrap().as_str() {
        "hub" => {
            let ip = get_env_var_strict("HOST", Some(DEFAULT_HOST));
            let port = get_env_var_strict("PORT", Some(DEFAULT_PORT));
            LaunchArgs::Hub(ip, port)
        }
        "node" => {
            let ip = get_env_var_strict("HUB_IP", Some(DEFAULT_HOST));
            let port = get_env_var_strict("HUB_PORT", Some(DEFAULT_PORT));

            let reconnect = args.contains(&"--reconnect".to_owned());
            let enable_sessions = args.contains(&"who".to_owned());
            let enable_wg_peers = args.contains(&"wg".to_owned());

            LaunchArgs::Node(
                ip,
                port,
                NodeConfig {
                    reconnect,
                    enable_sessions,
                    enable_wg_peers,
                },
            )
        }
        "help" => {
            println!("{USAGE}");
            process::exit(0);
        }
        _ => LaunchArgs::Client(args.into_iter().skip(1).collect::<Vec<_>>().join(" ")),
    };

    if let Err(e) = launch(launch_args) {
        eprintln!("{e}");
        process::exit(1);
    } else {
        process::exit(0);
    }
}
