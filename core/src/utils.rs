use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use std::{env, process};
use users::get_current_uid;

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
    let uid = get_current_uid();
    if uid == 0 {
        return "/run/secmon.sock".to_owned();
    } else {
        return format!("/run/user/{uid}/secmon.sock");
    }
}

/// Returns whether a hub-node response is a partial response that will have subsequent streaming response.
pub fn is_streaming_response(response: &Response) -> bool {
    // we don't use catch-all to ensure this method is updated when a new response is added
    match response {
        Response::ResultStream(ResultStatus::Pending, _) => true,
        Response::ResultStream(_, _)
        | Response::KeepAlive
        | Response::Connect(_)
        | Response::NodeState(_)
        | Response::NodeUpdate(_)
        | Response::Result(..) => false,
    }
}
