use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, rt, web};
use anyhow::Result;
use std::env;
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{self, Sender};

use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::models::node::NodeUpdate;
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var_strict, get_socket_path};

mod models;
mod routes;
mod utils;
use crate::models::{DEFAULT_IP, DEFAULT_PORT};

const USAGE: &str = "Usage:
  secmon-http                       launch http server
  secmon-http help                  show this help message

Environment:
  SERVER_IP=<ip> SERVER_PORT=<port> (default: 127.0.0.1:9993)";

/// Subscribes to hub for node updates, then broadcasts updates.
fn handle_node_update(upd_s: Sender<(u32, NodeUpdate)>) -> Result<()> {
    let mut stream = UnixStream::connect(get_socket_path())?;
    println!("Connected to hub daemon via unix socket");

    stream.write(&ClientCommand::Subscribe)?;

    loop {
        let resp = stream.read::<ClientResponse>()?;
        match resp {
            ClientResponse::NodeUpdate(serial, data) => {
                let _ = upd_s.send((serial, data)); // ignore error when nobody listening
            }
            _ => {} // should not happen
        }
    }
}

/// Handles websocket that subscribes to node updates.
async fn handle_subscribe(
    req: HttpRequest,
    stream: web::Payload,
    upd_s_d: web::Data<Sender<(u32, NodeUpdate)>>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, _) = actix_ws::handle(&req, stream)?;

    let mut upd_r = upd_s_d.subscribe();

    rt::spawn(async move {
        loop {
            match upd_r.recv().await {
                Ok(update) => {
                    session
                        .text(serde_json::to_string(&update).unwrap())
                        .await
                        .unwrap();
                }
                Err(RecvError::Lagged(_)) => continue, // ignore; just keep receiving
                Err(RecvError::Closed) => {
                    panic!("node update producer closed; this should not happen")
                }
            }
        }
    });

    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let ip = get_env_var_strict("SERVER_IP", Some(DEFAULT_IP));
    let port = get_env_var_strict("SERVER_PORT", Some(DEFAULT_PORT));

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    } else if args.len() != 1 {
        eprintln!("Invalid command; Use 'secmon-http help' for help");
        process::exit(1);
    }

    let (upd_s, _) = broadcast::channel::<(u32, NodeUpdate)>(32);
    let upd_s_t = upd_s.clone();
    thread::spawn(move || {
        let mut last_error = String::new();

        loop {
            let upd_s = upd_s_t.clone();
            if let Err(e) = handle_node_update(upd_s) {
                let e_str = format!("{e}");
                if last_error != e_str {
                    println!("{e_str}");
                    last_error = e_str;
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    println!("Listening on {ip}:{port} for http requests");

    let upd_s_d = web::Data::new(upd_s);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::clone(&upd_s_d))
            .route("/subscribe", web::get().to(handle_subscribe))
            .service(routes::get_list)
            .service(routes::post_execute)
            .service(routes::get_node)
    })
    .bind((ip, port))?
    .run()
    .await
}
