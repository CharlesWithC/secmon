use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};

use secmon::models::hub::Node;
use secmon::models::packet::{Command, Response};

use crate::models::HttpError;
use crate::utils;

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

#[get("/{node}")]
pub async fn get_node(path: web::Path<String>) -> impl Responder {
    let node_query = path.into_inner();
    let node: Node;
    parse_result!(utils::find_node(node_query.clone()), node);
    HttpResponse::Ok().json(node)
}

#[post("/{node}/execute")]
pub async fn post_execute(
    request: HttpRequest,
    path: web::Path<String>,
    command: web::Json<Command>,
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

    let node_query = path.into_inner();
    let node: Node;
    parse_result!(utils::find_node(node_query.clone()), node);
    let resp: Response;
    parse_result!(
        utils::raw_command(&node, command.to_owned(), expires_in),
        resp
    );
    HttpResponse::Ok().json(resp)
}
