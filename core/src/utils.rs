use anyhow::{Result, anyhow, bail};
use crossbeam_channel::Receiver;
use serde::de::DeserializeOwned;
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;
use std::{env, process, thread};
use users::get_current_uid;

use crate::config::{ClientConfig, NodeOrderKey};
use crate::logging::vprintln;
use crate::models::hub::Node;
use crate::models::packet::{Response, ResultStatus};

/// Returns the length for display for a `Result<Vec<_>>` value.
///
/// If the `Result` is an `Err`, then returns `-1`;
/// otherwise, returns the length of the vector.
pub fn get_display_len<T, E>(r: &Result<Vec<T>, E>) -> i32 {
    r.as_ref().map(|v| v.len() as i32).unwrap_or(-1)
}

/// Returns parsed env var value for `key`.
///
/// If env var is missing, then returns `default`.
///
/// If env var cannot be parsed, then returns an error.
pub fn get_env_var<T: FromStr + ToString>(
    key: &str,
    default: Option<T>,
) -> Result<Option<T>, <T as FromStr>::Err> {
    match env::var(key) {
        Ok(val) => match val.parse::<T>() {
            Ok(parsed) => Ok(Some(parsed)),
            Err(e) => Err(e),
        },
        Err(_) => Ok(default),
    }
}

/// Returns parsed env var value for `key`.
///
/// If env var is missing, then returns `default`.
///
/// If env var is missing and `default` is `None`, or
/// if env var cannot be parsed, then exit with code 1.
pub fn get_env_var_strict<T: FromStr + ToString>(key: &str, default: Option<T>) -> T
where
    T::Err: std::fmt::Debug,
{
    get_env_var(key, default)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse {key}: {:?}", e);
            process::exit(1);
        })
        .unwrap_or_else(|| {
            eprintln!("Missing env var: {key}");
            process::exit(1);
        })
}

/// Returns an Iterator to the Reader of the lines of the file.
///
/// This function is conveniently copied from [Rust By Example](https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html).
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Returns the socket path that should be used for client communication.
pub fn get_socket_path() -> String {
    match get_current_uid() {
        0 => "/run/secmon.sock".to_owned(),
        uid @ _ => format!("/run/user/{uid}/secmon.sock"),
    }
}

/// Returns whether a hub-node response is a partial response that will have subsequent streaming response.
pub fn is_streaming_response(response: &Response) -> bool {
    // we don't use catch-all to ensure this method is updated when a new response is added
    match response {
        Response::ResultStream {
            status: ResultStatus::Pending,
            ..
        } => true,
        Response::ResultStream { status: _, .. }
        | Response::KeepAlive
        | Response::Connect { .. }
        | Response::NodeState(_)
        | Response::NodeUpdate(_)
        | Response::Result { .. } => false,
    }
}

/// Spawns a thread to check if anything is sent over `channel` within `deadline`.
///
/// If yes, then call `callback_success`. Otherwise, call `callback_fail`.
pub fn deadline_watchdog<F, G>(
    rx: Receiver<()>,
    deadline: Duration,
    callback_success: F,
    callback_fail: G,
) -> ()
where
    F: FnOnce() + Send + 'static,
    G: FnOnce() + Send + 'static,
{
    thread::spawn(move || {
        match rx.recv_timeout(deadline) {
            Ok(_) => callback_success(), // success if anything is received
            Err(_) => callback_fail(), // disconnect (rx is dropped) or timeout (deadline is reached)
        }
    });
}

/// Sort nodes in place based on client config.
pub fn sort_nodes(nodes: &mut Vec<Node>, config: &ClientConfig) -> () {
    let cmp_group_base = |a: &Node, b: &Node| -> Ordering { a.group.cmp(&b.group) };

    let cmp_node_key: fn(&Node, &Node) -> Ordering = match config.node_order_key {
        NodeOrderKey::Serial => |a, b| a.serial.cmp(&b.serial),
        NodeOrderKey::Address => |a, b| a.address.cmp(&b.address),
        NodeOrderKey::Hostname => |a, b| a.hostname.cmp(&b.hostname),
    };

    let node_index_max = config.node_order_override.len();
    let get_node_index = |x: &Node| -> &usize {
        config
            .node_order_override
            .get(&x.hostname)
            .unwrap_or(&node_index_max)
    };
    let cmp_node_override =
        |a: &Node, b: &Node| -> Ordering { get_node_index(a).cmp(get_node_index(b)) };

    let group_index_max = config.group_order_override.len();
    let get_group_index = |x: &Node| -> &usize {
        match x.group {
            Some(ref group) => config
                .group_order_override
                .get(group)
                .unwrap_or(&group_index_max),
            None => &group_index_max,
        }
    };
    let cmp_group_override =
        |a: &Node, b: &Node| -> Ordering { get_group_index(a).cmp(get_group_index(b)) };

    nodes.sort_by(|a, b| {
        Ordering::Equal
            .then_with(|| cmp_group_override(a, b))
            .then_with(|| cmp_group_base(a, b))
            .then_with(|| cmp_node_override(a, b))
            .then_with(|| cmp_node_key(a, b))
    });
}

/// Returns tuple `(config_path, verbose)` parsed from command line arguments.
///
/// If `-c` is set, then `config_path` is `Some(path)`, otherwise `None`.
///
/// If `-v` is set, then `verbose` is `true`, otherwise `false`.
pub fn parse_generic_args(args: &Vec<String>) -> Result<(Option<String>, bool)> {
    let mut config_path: Option<String> = None;
    let mut verbose = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "-c" || arg == "--config" {
            if !config_path.is_none() {
                bail!("Duplicate '-c' argument detected. Provide only one config file.");
            }

            let Some(path) = it.next() else {
                bail!("Invalid '-c' argument. Provide a config file.");
            };
            match Path::new(path).try_exists() {
                Ok(false) => bail!("Unable to find config file at {path}"),
                Err(e) => bail!("Unable to read config file at {path}: {e}"),
                _ => config_path = Some(path.clone()),
            }
        }
        if arg == "-v" || arg == "--verbose" {
            verbose = true;
        }
    }

    Ok((config_path, verbose))
}

/// Returns sanitized generic command line arguments.
///
/// This removes `-c` and `-v` arguments from `args`, assuming `-c` is followed by config path.
///
/// If an unrecognized argument is detected, prints an error and exits program.
pub fn sanitize_generic_args(args: &Vec<String>) -> Vec<String> {
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
            _ => {
                if a.starts_with("-") {
                    println!("Invalid argument: {a:?}");
                    process::exit(1);
                }
                true
            }
        }
    });
    args
}

/// Returns parsed toml configuration given an optional path.
///
/// If `path` is `None`, then try default locations, i.e.
/// `$XDG_CONFIG_HOME/secmon/config.toml` and `/etc/secmon/config.toml`.
///
/// A generic type `T` is used to allow loading different formats of config.
pub fn read_config<T: DeserializeOwned>(path: Option<String>) -> Result<T> {
    let mut config_path = path;

    if config_path.is_none() {
        let mut try_paths: Vec<String> = vec![];

        if let Ok(Some(config_home)) = get_env_var::<String>("XDG_CONFIG_HOME", None)
            && !config_home.is_empty()
        {
            try_paths.push(config_home + "/secmon/config.toml");
        }
        if let Ok(Some(home)) = get_env_var::<String>("HOME", None)
            && !home.is_empty()
        {
            try_paths.push(home + "/.config/secmon/config.toml");
        }
        try_paths.push("/etc/secmon/config.toml".to_owned());

        for path in try_paths.iter() {
            // we treat no-access files as invisible so we use naive `exists`
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

    toml::from_str::<T>(&config_raw).map_err(|e| anyhow!("Unable to parse config file: {e}"))
}

/// Initialize program from generic arguments, specifically,
/// - detects configuration file, then parses and sets it in `config`
/// - detects verbose mode, then sets it in `secmon::logging::VERBOSE`
pub fn init_from_generic_args<T: DeserializeOwned + std::fmt::Debug>(
    args: &Vec<String>,
    config: &OnceLock<T>,
) -> Result<()> {
    let (config_path, _verbose) = parse_generic_args(args)?;
    let _config = read_config::<T>(config_path)?;

    crate::logging::VERBOSE
        .set(_verbose)
        .map_err(|_| panic!("Error when setting verbose mode"));

    vprintln!("{:?}", _config);

    config
        .set(_config)
        .map_err(|_| panic!("Error when setting config"));

    Ok(())
}
