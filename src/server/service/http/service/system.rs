use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use actix_web::{web, HttpResponse, Responder};

use super::dto;

#[actix_web::post("/system/add_proxy")]
async fn add_proxy(
    proxy_mp: web::Data<Arc<Mutex<BTreeMap<String, String>>>>,
    web::Json(proxy): web::Json<dto::Proxy>,
) -> impl Responder {
    let mut proxies = proxy_mp.lock().unwrap();
    proxies.insert(proxy.path.clone(), proxy.url.clone());
    HttpResponse::Ok().finish()
}

#[actix_web::delete("/system/remove_proxy")]
pub async fn remove_proxy(
    proxy_mp: web::Data<Arc<Mutex<BTreeMap<String, String>>>>,
    web::Json(proxy): web::Json<dto::Proxy>,
) -> impl Responder {
    let mut proxies = proxy_mp.lock().unwrap();
    proxies.remove(&proxy.path);
    HttpResponse::Ok().finish()
}

#[actix_web::get("/system/list_proxies")]
pub async fn list_proxies(
    proxy_mp: web::Data<Arc<Mutex<BTreeMap<String, String>>>>,
) -> impl Responder {
    let proxies = proxy_mp.lock().unwrap();

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
