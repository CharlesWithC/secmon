use crossbeam_channel::Receiver;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use std::{env, process};
use users::get_current_uid;

use crate::config::{ClientConfig, NodeOrderKey};
use crate::models::hub::Node;
use crate::models::packet::{Response, ResultStatus};

/// `println!` wrapper for verbose mode.
#[doc(hidden)]
#[macro_export]
macro_rules! __vprintln {
    ($($arg:tt)*) => {
        // note: VERBOSE is not set if -v/--verbose is not passed
        if *secmon::config::VERBOSE.get().unwrap_or(&false) {
            println!($($arg)*);
        }
    };
}
pub use crate::__vprintln as vprintln;

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
