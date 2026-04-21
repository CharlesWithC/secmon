use anyhow::Result;
use std::process::Output;
use std::str::FromStr;
use std::{env, process};

/// Returns parsed output of a command execution.
///
/// If command did not succeed, or an error occurs
/// when parsing, raises a string-based error.
pub fn parse_command_output(
    program: &str,
    output: Result<Output, std::io::Error>,
) -> Result<String, String> {
    let output = output.map_err(|e| format!("Failed to execute '{program}': {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Command '{program}' did not succeed: {}",
            str::from_utf8(&output.stderr)
                .map(|v| v.trim())
                .unwrap_or("Unable to parse stderr")
        ));
    }

    let parsed_output =
        str::from_utf8(&output.stdout).map_err(|e| format!("Unable to parse stdout: {e}"))?;

    Ok(parsed_output.to_owned())
}

/// Returns parsed env var value for `key`.
///
/// If env var is missing, then returns `default`.
///
/// If env var cannot be parsed, then returns `None`.
pub fn get_env_var<T: FromStr + ToString>(key: &str, default: Option<T>) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    let val = env::var(key);
    if let Err(_) = val {
        if let None = default {
            eprintln!("Missing env var: {key}");
        }
        return default;
    }

    let parsed_val = val.unwrap().parse::<T>();
    if let Err(e) = parsed_val {
        eprintln!("Failed to parse {key}: {:?}", e);
        return None;
    }

    Some(parsed_val.unwrap())
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
    get_env_var(key, default).unwrap_or_else(|| process::exit(1))
}
