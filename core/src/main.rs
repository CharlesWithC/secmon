use anyhow::{Result, anyhow};
use std::fs;
use std::{env, process};

mod hub;
mod models;
mod node;
mod traits;
mod utils;
use crate::models::{DEFAULT_IP, DEFAULT_PORT, HubConfig, LaunchArgs, NodeConfig};
use crate::utils::{get_env_var_strict, get_socket_path};

const USAGE: &str = "Usage:
  secmon hub                        launch hub daemon
  secmon node [who] [wg] [auth]     launch node daemon
    [--reconnect]
  secmon help                       print this help message

Utility commands:
  secmon subscribe                  subscribe to node state atomic updates
  secmon list [sorted]              list all connected nodes
  secmon <node> <command-label>     execute an allowed command

  <node> can be serial, address, hostname, or \"-\" for all connected nodes.
  Due to sync design, each node can only handle one command at a time.

Environment:
  hub:      BIND_IP=<ip> BIND_PORT=<port>       (default: 127.0.0.1:9992)
            REMOTE_EXEC_TIMEOUT=<seconds>       (default: 300)
                when to timeout a remote command execution
            DISCONNECT_GRACE_PERIOD=<seconds>   (default: 300)
                when to remove a disconnected node, if not replaced
            ASSUME_HOSTNAME_UNIQUE=<true|false> (default: true)
                if true, reconnected node would replace disconnected node
                otherwise, reconnected node would be considered a new node
  node:     HUB_IP=<ip> HUB_PORT=<port>         (default: 127.0.0.1:9992)
            COMMAND_ALLOWLIST_FILE=<path>       (default: none)

COMMAND_ALLOWLIST_FILE:
  A file containing commands that may be executed by hub.
  Separate label and command with '=', and provide one pair in each line.
  Label must not contain '=', and command must finish in one line.
  Examples:
    LABEL=COMMAND
    update=apt update -y
    reboot=shutdown -r";

fn launch(launch_args: LaunchArgs) -> Result<()> {
    match launch_args {
        LaunchArgs::Hub(ip, port, hub_config) => {
            let socket_path = get_socket_path();
            if fs::exists(&socket_path)
                .map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))?
            {
                fs::remove_file(&socket_path)
                    .map_err(|e| anyhow!("Unable to unlink {socket_path}: {e}"))?;
            }

            crate::hub::main_daemon(hub_config, ip, port, socket_path)?;
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
            let ip = get_env_var_strict("BIND_IP", Some(DEFAULT_IP));
            let port = get_env_var_strict("BIND_PORT", Some(DEFAULT_PORT));

            let remote_exec_timeout = get_env_var_strict("REMOTE_EXEC_TIMEOUT", Some(300));
            let disconnect_grace_period = get_env_var_strict("DISCONNECT_GRACE_PERIOD", Some(300));
            let assume_hostname_unique = get_env_var_strict("ASSUME_HOSTNAME_UNIQUE", Some(true));

            LaunchArgs::Hub(
                ip,
                port,
                HubConfig {
                    remote_exec_timeout,
                    disconnect_grace_period,
                    assume_hostname_unique,
                },
            )
        }
        "node" => {
            let ip = get_env_var_strict("HUB_IP", Some(DEFAULT_IP));
            let port = get_env_var_strict("HUB_PORT", Some(DEFAULT_PORT));

            let reconnect = args.contains(&"--reconnect".to_owned());
            let enable_sessions = args.contains(&"who".to_owned());
            let enable_wg_peers = args.contains(&"wg".to_owned());
            let enable_auth_log = args.contains(&"auth".to_owned());

            LaunchArgs::Node(
                ip,
                port,
                NodeConfig {
                    reconnect,
                    enable_sessions,
                    enable_wg_peers,
                    enable_auth_log,
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
