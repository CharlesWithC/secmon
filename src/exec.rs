use anyhow::Result;
use std::ffi::OsStr;
use std::process::Command;

/// Executes `program` with `args` and returns parsed output.
///
/// If an error occurs, returns a string-based error.
pub fn exec<I, S>(program: S, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr> + std::fmt::Display,
{
    let output = Command::new(&program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute '{program}': {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Command '{program}' did not succeed: {}",
            str::from_utf8(&output.stderr)
                .map(|v| v.trim())
                .unwrap_or("Unable to parse stderr")
        ));
    }

    let parsed_output =
        str::from_utf8(&output.stdout).map_err(|e| format!("Unable to parse stdout: {}", e))?;

    Ok(parsed_output.to_owned())
}
