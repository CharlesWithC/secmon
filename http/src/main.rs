use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, rt, web};
use anyhow::Result;
use std::env;
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{self, Sender};

use secmon::GENERIC_OPTIONS;
use secmon::config::get_config;
use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_socket_path, init_from_generic_args, sanitize_generic_args};

mod models;
mod routes;
use crate::models::BodyNodeUpdate;
use secmon_http::config::{CONFIG, HttpConfig};

const USAGE: &str = const_format::formatcp!(
    "Usage:
  secmon-http                       launch http server
  secmon-http help                  show this help message

Options:
  {GENERIC_OPTIONS}"
);

/// Subscribes to hub for node updates, then broadcasts updates.
fn handle_node_update(upd_s: Sender<BodyNodeUpdate>) -> Result<()> {
    let mut stream = UnixStream::connect(get_socket_path())?;
    println!("Connected to hub daemon via unix socket");

    stream.write(&ClientCommand::Subscribe)?;

    loop {
        let resp = stream.read::<ClientResponse>()?;
        match resp {
            ClientResponse::NodeUpdate { node_serial, data } => {
                let _ = upd_s.send(BodyNodeUpdate { node_serial, data }); // ignore error when nobody listening
            }
            _ => {} // should not happen
        }
    }
}

/// Handles websocket that subscribes to node updates.
async fn handle_subscribe(
    req: HttpRequest,
    stream: web::Payload,
    upd_s_d: web::Data<Sender<BodyNodeUpdate>>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, _) = actix_ws::handle(&req, stream)?;

    let mut upd_r = upd_s_d.subscribe();

    rt::spawn(async move {
        loop {
            match upd_r.recv().await {
                Ok(update) => match serde_json::to_string(&update) {
                    Ok(update_json) => match session.text(update_json).await {
                        Ok(..) => {}     // all good, keep going
                        Err(_) => break, // connection closed, break
                    },
                    Err(e) => {
                        eprintln!("Failed to serialize node update to json: {e}");
                    }
                },
                Err(RecvError::Lagged(_)) => {} // ignore; just keep receiving
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
    let args: Vec<String> = env::args().collect();
    let sanitized_args = sanitize_generic_args(&args);
    if sanitized_args.len() == 2 && sanitized_args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    } else if sanitized_args.len() != 1 {
        eprintln!("Unrecognized command: {}\n\n{USAGE}", sanitized_args[1]);
        process::exit(1);
    }

    init_from_generic_args(&args, &CONFIG).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    let HttpConfig {
        bind_ip: ip,
        bind_port: port,
    } = get_config!(http);

    let (upd_s, _) = broadcast::channel::<BodyNodeUpdate>(32);
    let upd_s_t = upd_s.clone();
    thread::spawn(move || {
        let mut last_error = String::new();

        loop {
            let upd_s = upd_s_t.clone();
            if let Err(e) = handle_node_update(upd_s) {
                let e_str = format!("{e}");
                if last_error != e_str {
                    println!("Unable to connect to hub daemon: {e_str}");
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
            .service(routes::get_nodes)
            .service(routes::post_execute)
    })
    .bind((*ip, *port))?
    .run()
    .await
}
