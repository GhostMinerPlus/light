use actix_web::{web, HttpResponse, Responder};

use crate::api::Context;

use super::dto;

#[actix_web::post("/system/add_proxy")]
async fn add_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    let mut proxies = Context::as_ref().proxy.lock().unwrap();
    proxies.insert(proxy.path.clone(), proxy.url.clone());
    HttpResponse::Ok().finish()
}

#[actix_web::delete("/system/remove_proxy")]
pub async fn remove_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    let mut proxies = Context::as_ref().proxy.lock().unwrap();
    proxies.remove(&proxy.path);
    HttpResponse::Ok().finish()
}

#[actix_web::get("/system/list_proxies")]
pub async fn list_proxies() -> impl Responder {
    let proxies = Context::as_ref().proxy.lock().unwrap();

    let mut list = Vec::with_capacity(proxies.len());
    for proxy in &*proxies {
        list.push(dto::Proxy {
            path: proxy.0.clone(),
            url: proxy.1.clone(),
        });
    }
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&list).unwrap())
}