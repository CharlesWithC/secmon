use actix_web::{HttpResponse, Responder, get, post, web};

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
pub async fn post_execute(path: web::Path<String>, command: web::Json<Command>) -> impl Responder {
    if utils::is_streaming_command(&command) {
        return HttpResponse::BadRequest().json(HttpError {
            error: "Streaming response is not supported.".to_owned(),
        });
    }
    let node_query = path.into_inner();
    let node: Node;
    parse_result!(utils::find_node(node_query.clone()), node);
    let resp: Response;
    parse_result!(utils::raw_command(&node, command.to_owned()), resp);
    HttpResponse::Ok().json(resp)
}
