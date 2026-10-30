use anyhow::{Result, anyhow, bail};
use gethostname::gethostname;
use getrandom;
use snow::Builder;
use std::fs;
use std::path::Path;
use std::{env, process};

mod config;
mod crypto;
mod hub;
mod node;
use crate::config::{CONFIG, Config, VERBOSE};
use crate::crypto::{NOISE_KEYLEN, Key, NOISE_PATTERN};
use secmon::utils::{get_env_var, get_socket_path};

const USAGE: &str = "Usage:
  secmon hub                        launch hub daemon
  secmon node                       launch node daemon
  secmon help                       show this help message

Utility commands:
  secmon genkey <pair|psk>          generate keypair or preshared key
  secmon subscribe                  subscribe to node state atomic updates
  secmon <list|->                   list all recently connected nodes
  secmon <node>                     show info about a recently connected node
  secmon <node> <command-label>     execute an allowed command

  <node> can be serial, address, hostname. Use \"-\" for all connected nodes.
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

/// Parses and loads configuration given an optional path.
///
/// If `path` is `None`, then try default locations, i.e.
/// `$XDG_CONFIG_HOME/secmon/config.toml` and `/etc/secmon/config.toml`.
fn load_config(path: Option<String>) -> Result<()> {
    let mut config_path = path;

    if config_path.is_none() {
        let mut try_paths: Vec<String> = vec![];

        if let Ok(Some(config_home)) = get_env_var::<String>("XDG_CONFIG_HOME", None) {
            try_paths.push(config_home + "/secmon/config.toml");
        } else if let Ok(Some(home)) = get_env_var::<String>("HOME", None) {
            try_paths.push(home + "/.config/secmon/config.toml");
        }
        try_paths.push("/etc/secmon/config.toml".to_owned());

        for path in try_paths.iter() {
            if Path::new(path).exists() {
                config_path = Some(path.clone());
                break;
            }
        }
    }

    let Some(config_path) = config_path else {
        bail!("Unable to find a config file. Specify one with '-c' argument.");
    };

    let config_raw = fs::read_to_string(&config_path)
        .map_err(|e| anyhow!("Unable to read config file at {config_path}: {e}"))?;

    println!("Using config file at {config_path}");

    let mut parsed_config: Config =
        toml::from_str(&config_raw).map_err(|e| anyhow!("Unable to parse config file: {e}"))?;
    if parsed_config.node.hostname.is_empty() {
        parsed_config.node.hostname = gethostname().to_string_lossy().into_owned();
    }

    vprintln!("{:?}", parsed_config);

    CONFIG
        .set(parsed_config)
        .map_err(|_| panic!("Error when initializing config"))
}

/// Parses and handles options in args, specifically,
/// - loads configuration file
/// - detects verbose mode
fn handle_args_options(args: &Vec<String>) -> Result<()> {
    let mut config_path: Option<String> = None;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "-c" || arg == "--config" {
            if !config_path.is_none() {
                bail!("Duplicate '-c' argument detected. Provide only one config file.");
            }

            let Some(path) = it.next() else {
                bail!("Invalid '-c' argument. Provide a config file.");
            };
            if !Path::new(path).exists() {
                bail!("Config file does not exist: {path}");
            }
            config_path = Some(path.clone());
        }
        if arg == "-v" || arg == "--verbose" {
            VERBOSE.set(true).ok();
        }
    }

    load_config(config_path)?;

    Ok(())
}

/// Return args with options removed.
fn sanitize_args(args: &Vec<String>) -> Vec<String> {
    let mut args = args.clone();
    let mut skip_next = false;
    args.retain(|a| {
        if std::mem::take(&mut skip_next) {
            return false; // value belonging to -c/--config
        }
        match a.as_str() {
            "-v" | "--verbose" => false,
            "-c" | "--config" => {
                skip_next = true;
                false
            }
            _ => true,
        }
    });
    return args;
}

/// Wrapped "real" main to return errors if any.
fn launch(args: &Vec<String>) -> Result<()> {
    match args.get(1).unwrap().as_str() {
        "hub" => {
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

    if args.len() < 2 {
        println!("{USAGE}");
        process::exit(1);
    }

    match handle_simple_command(&args) {
        Ok(true) => {
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
        _ => {} // not a simple command! keep going!
    }

    handle_args_options(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let args = sanitize_args(&args);
    if let Err(e) = launch(&args) {
        eprintln!("{e}");
        process::exit(1);
    } else {
        process::exit(0);
    }
}
