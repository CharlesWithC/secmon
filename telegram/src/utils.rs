use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use secmon::models::hub::{ClientCommand, ClientResponse, Node};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::get_socket_path;

/// Returns a node based on serial.
///
/// This method tries to find the node in cached data if `use_cache=true`,
/// and requests data from hub if the node cannot be found.
pub fn find_node(
    serial: u32,
    nodes_mutex: &Arc<Mutex<Vec<Node>>>,
    use_cache: bool,
) -> Result<Node> {
    // try to find node in cache
    let mut guard = nodes_mutex.lock().unwrap();
    let ref mut nodes = *guard;
    let node_opt = nodes.iter().find(|node| node.serial == serial);
    if use_cache && let Some(node) = node_opt {
        return Ok(node.clone());
    }

    // request data from hub
    // note: we keep the guard to ensure cache integrity and since hub responds very quickly
    let resp = execute_command(&ClientCommand::FindNode(serial.to_string()))?;
    match resp {
        ClientResponse::Node(node) => {
            nodes.push(node.clone());
            Ok(node)
        }
        ClientResponse::Failure(e) => Err(anyhow!("Unable to find node: {e}")),
        _ => Err(anyhow!("Unable to find node: Hub sent an invalid response")),
    }
}

/// Executes a command on hub and returns the response.
pub fn execute_command(command: &ClientCommand) -> Result<ClientResponse> {
    match UnixStream::connect(get_socket_path()) {
        Ok(ref mut stream) => {
            stream.write(command)?;
            stream.read::<ClientResponse>().map(|resp| Ok(resp))?
        }
        Err(e) => Err(anyhow!("Unable to connect to hub daemon: {e}")),
    }
}
