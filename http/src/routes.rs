use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};

use secmon::models::hub::Node;
use secmon::models::packet::{Request, Response};

use crate::models::HttpError;
use secmon_http::utils;

macro_rules! parse_result {
    ( $result:expr, $value:expr ) => {
        if let Err(e) = $result {
            return HttpResponse::InternalServerError().json(HttpError {
                error: format!("{}", e),
            });
        }
        $value = $result.unwrap();
    };
}

macro_rules! bad_request {
    ( $err:expr ) => {
        return HttpResponse::BadRequest().json(HttpError {
            error: $err.to_owned(),
        });
    };
}

#[get("/list")]
pub async fn get_list() -> impl Responder {
    let nodes: Vec<Node>;
    parse_result!(utils::list_nodes(), nodes);
    HttpResponse::Ok().json(nodes)
}

#[get("/{nodes}")]
pub async fn get_nodes(path: web::Path<String>) -> impl Responder {
    let node_query = path.into_inner();
    let node: Vec<Node>;
    parse_result!(utils::find_nodes(node_query.clone()), node);
    HttpResponse::Ok().json(node)
}

#[post("/{serial}/execute")]
pub async fn post_execute(
    request: HttpRequest,
    path: web::Path<String>,
    command: web::Json<Request>,
) -> impl Responder {
    if utils::is_streaming_command(&command) {
        bad_request!("Streaming response is not supported.");
    }

    let mut expires_in = 0;
    if let Some(expires_in_header) = request.headers().get("Expires-In") {
        match expires_in_header.to_str() {
            Ok(expires_in_str) => match expires_in_str.parse::<u64>() {
                Ok(expires_in_int) => expires_in = expires_in_int,
                Err(e) => {
                    bad_request!(format!("Unable to parse Expires-In header: {e}"));
                }
            },
            Err(e) => {
                bad_request!(format!("Unable to parse Expires-In header: {e}"));
            }
        }
    }

    let serial_str = path.into_inner();
    match serial_str.parse::<u32>() {
        Ok(serial) => {
            let resp: Response;
            parse_result!(
                utils::raw_command_with_serial(serial, command.to_owned(), expires_in),
                resp
            );
            HttpResponse::Ok().json(resp)
        }
        Err(_) => {
            bad_request!(format!("Invalid node serial: {serial_str}"));
        }
    }
}
