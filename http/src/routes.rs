use actix_web::{HttpResponse, Responder, get, web};

use secmon::models::hub::Node;

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
