use std::str::FromStr;
use std::{env, process};

/// Returns the length for display for an `Option<Result<Vec<_>, _>` type value.
///
/// If the value is `None`, or `Result` is an `Err`, then returns `-1`;
/// otherwise, returns the length of the vector.
pub fn get_display_len<T, E>(v: &Option<Result<Vec<T>, E>>) -> i32 {
    v.as_ref()
        .map(|r| r.as_ref().map(|v| v.len() as i32).unwrap_or(-1))
        .unwrap_or(-1)
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
