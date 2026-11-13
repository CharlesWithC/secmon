use anyhow::{Result, anyhow, bail};
use getrandom;
use snow::Builder;
use std::fs;
use std::{env, process};

mod hub;
mod node;
use secmon::config::{CONFIG, get_config};
use secmon::crypto::{Key, NOISE_KEYLEN, NOISE_PATTERN};
use secmon::logging::VERBOSE;
use secmon::utils::{get_socket_path, init_from_generic_args, sanitize_generic_args};

const USAGE: &str = "Usage:
  secmon hub                        launch hub daemon
  secmon node                       launch node daemon
  secmon help                       show this help message

Utility commands:
  secmon genkey <pair|psk>          generate keypair or preshared key
  secmon subscribe                  subscribe to node state atomic updates
  secmon <list|->                   print info of all nodes
  secmon <node>                     print info of one node
  secmon <node|-> <command-label>   execute a command on one node or all nodes
  secmon <group>                    print info of all nodes in group
  secmon <group> <command-label>    execute a command on all nodes in group

  <node> can be serial, address, hostname. \"-\" refers to all connected nodes.
  <group> takes precedence if individual <node> can be identified by group name.
  Due to sync design, each node can only handle one command at a time.

Options:
  -c, --config <file>               specify a config file, otherwise tries:
                                    $XDG_CONFIG_HOME/secmon/config.toml,
                                    /etc/secmon/config.toml in order by default
  -v, --verbose                     verbose mode
";

/// Handles simple commands that do not involve options.
///
/// Returns `Ok(true)` if command is handled, `Ok(false)` if command is not handled, and `Err` is an error occurred.
fn handle_simple_command(args: &Vec<String>) -> Result<bool> {
    match args.get(1).unwrap().as_str() {
        "genkey" => {
            match args
                .get(2)
                .ok_or(anyhow!(
                    "Specify the type of key to generate: 'pair' or 'psk'."
                ))?
                .as_str()
            {
                "pair" => {
                    let keypair = Builder::new(NOISE_PATTERN.parse().unwrap())
                        .generate_keypair()
                        .unwrap();
                    println!("Private: {}", Key::from(keypair.private));
                    println!("Public: {}", Key::from(keypair.public));
                    Ok(true)
                }
                "psk" => {
                    let mut buf = [0u8; NOISE_KEYLEN];
                    getrandom::fill(&mut buf).expect("Failed to fill random buffer");
                    println!("Preshared: {}", Key::from(buf));
                    Ok(true)
                }
                kt @ _ => {
                    bail!("{kt:?} is not a valid type of key. Choose from 'pair' or 'psk'.");
                }
            }
        }
        "help" => {
            println!("{USAGE}");
            Ok(true)
        }
        _ => Ok(false), // to be handled by `launch`
    }
}

/// Wrapped "real" main to return errors if any.
fn launch(args: &Vec<String>) -> Result<()> {
    match args.get(1).unwrap().as_str() {
        "hub" => {
            let noise = &get_config!(hub).noise;
            if noise.enable {
                if !noise.private_key.exists() {
                    bail!("Invalid config: Noise protocol is enabled but `private_key` is not set");
                }
            }

            let socket_path = get_socket_path();
            if fs::exists(&socket_path)
                .map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))?
            {
                fs::remove_file(&socket_path)
                    .map_err(|e| anyhow!("Unable to unlink {socket_path}: {e}"))?;
            }

            crate::hub::main_daemon(socket_path)?;
        }
        "node" => {
            let noise = &get_config!(node).noise;
            if noise.enable {
                if !noise.private_key.exists() {
                    bail!("Invalid config: Noise protocol is enabled but `private_key` is not set");
                }
                if !noise.hub_public_key.exists() {
                    bail!(
                        "Invalid config: Noise protocol is enabled but `hub_public_key` is not set"
                    );
                }
            }

            crate::node::main()?;
        }
        _ => {
            let command = args[1..].join(" ");

            let socket_path = get_socket_path();
            if !fs::exists(&socket_path)
                .map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))?
            {
                return Err(anyhow!(
                    "{socket_path} does not exist, is hub daemon running?"
                ));
            }

            crate::hub::main_client(command, socket_path)?;
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let sanitized_args = sanitize_generic_args(&args);

    if args.len() < 2 {
        println!("{USAGE}");
        process::exit(1);
    }

    match handle_simple_command(&sanitized_args) {
        Ok(true) => {
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
        _ => {} // not a simple command! keep going!
    }

    init_from_generic_args(&args, &CONFIG, &VERBOSE).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    if let Err(e) = launch(&sanitized_args) {
        eprintln!("{e}");
        process::exit(1);
    } else {
        process::exit(0);
    }
}
