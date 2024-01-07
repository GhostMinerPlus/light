use actix_web::{web, HttpResponse, Responder};

use crate::util::Context;

use super::dto;

#[actix_web::post("/system/add_proxy")]
async fn add_proxy(
    ctx: web::Data<Context>,
    web::Json(proxy): web::Json<dto::Proxy>,
) -> impl Responder {
    let mut proxies = ctx.proxy.lock().unwrap();
    proxies.insert(proxy.path.clone(), proxy.url.clone());
    HttpResponse::Ok().finish()
}

#[actix_web::delete("/system/remove_proxy")]
pub async fn remove_proxy(
    ctx: web::Data<Context>,
    web::Json(proxy): web::Json<dto::Proxy>,
) -> impl Responder {
    let mut proxies = ctx.proxy.lock().unwrap();
    proxies.remove(&proxy.path);
    HttpResponse::Ok().finish()
}

#[actix_web::get("/system/list_proxies")]
pub async fn list_proxies(ctx: web::Data<Context>) -> impl Responder {
    let proxies = ctx.proxy.lock().unwrap();

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
