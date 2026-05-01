use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;
use std::time::UNIX_EPOCH;

use secmon::models::hub::{ClientCommand, ClientResponse, Node};
use secmon::models::packet::{Command, Response};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::get_socket_path;

macro_rules! match_response {
    ( $action:expr, $response:expr, $pattern:pat, $return:expr ) => {
        match $response {
            $pattern => Ok($return),
            ClientResponse::Error(e) => Err(anyhow!("Unable to {}: {}", $action, e)),
            _ => Err(anyhow!(
                "Unable to {}: Invalid hub daemon response: {}",
                $action,
                $response
            )),
        }
    };
}

/// Returns whether a client-hub request would result in streamed response.
pub fn is_streaming_command(command: &Command) -> bool {
    // we don't use catch-all to ensure this method is updated when a new response is added
    match command {
        Command::Execute { stream: true, .. } => true,
        Command::Execute { stream: false, .. } | Command::NodeState => false,
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

/// Returns result of finding a node.
pub fn find_node(query: String) -> Result<Node> {
    let resp = execute_command(&ClientCommand::FindNode { query })?;
    let node = match_response!("find node", resp, ClientResponse::Node(node), node)?;
    Ok(node)
}

/// Returns all nodes connected to hub.
pub fn list_nodes() -> Result<Vec<Node>> {
    let resp = execute_command(&ClientCommand::ListNodes)?;
    let nodes = match_response!("list nodes", resp, ClientResponse::Nodes(nodes), nodes)?;
    Ok(nodes)
}

/// Executes command on a specific node and returns result.
///
/// Note: Streaming result is not enabled. This will block until command execution completes.
pub fn raw_command(node: &Node, command: Command) -> Result<Response> {
    let raw_resp = execute_command(&ClientCommand::RawCommand {
        node_serial: node.serial,
        command,
        expire_time: UNIX_EPOCH,
    })?;
    let response = match_response!(
        "raw command",
        raw_resp,
        ClientResponse::RawResponse(response),
        response
    )?;
    Ok(response)
}
