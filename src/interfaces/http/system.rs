use actix_web::{web, Responder};

use crate::application::{self, dto};

pub(crate) async fn add_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    application::add_proxy(&proxy).await
}

pub(crate) async fn remove_proxy(web::Json(proxy): web::Json<dto::Proxy>) -> impl Responder {
    application::remove_proxy(&proxy.path).await
}

pub(crate) async fn list_proxies(web::Json(page): web::Json<dto::Page>) -> impl Responder {
    let list = application::list_proxies(&page).await;
    serde_json::to_string(&list)
}
