use actix_web::{web, Responder};

use crate::application::{self, dto};

#[actix_web::get("/system/add_proxy")]
async fn add_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    application::add_proxy(&proxy).await
}

#[actix_web::delete("/system/remove_proxy")]
pub async fn remove_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    application::remove_proxy(&proxy.path).await
}

#[actix_web::get("/system/list_proxies")]
pub async fn list_proxies(web::Json(page): web::Json<dto::Page>) -> impl Responder {
    let list = application::list_proxies(&page).await;
    serde_json::to_string(&list)
}
