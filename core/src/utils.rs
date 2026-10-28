use std::env;
use std::str::FromStr;
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
