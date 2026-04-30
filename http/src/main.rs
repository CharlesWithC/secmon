use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use std::env;
use std::process;

use secmon::utils::get_env_var_strict;

mod models;
use crate::models::{DEFAULT_IP, DEFAULT_PORT};

const USAGE: &str = "Usage:
  secmon-http                       launch http server
  secmon-http help                  show this help message

Environment:
  SERVER_IP=<ip> SERVER_PORT=<port> (default: 127.0.0.1:9993)";

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello http!")
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

    HttpServer::new(|| App::new().service(hello))
        .bind((ip, port))?
        .run()
        .await
}
