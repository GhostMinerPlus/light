pub(crate) mod system;

use actix_files::{Files, NamedFile};
use actix_http::{body::BoxBody, Payload};
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    dev::{forward_ready, Service, Transform},
    http::header::HeaderMap,
    web, Error, HttpRequest, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use reqwest::StatusCode;
use std::future::{ready, Ready};

use crate::App;

pub(crate) fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/system/add_proxy", web::post().to(system::add_proxy))
        .route(
            "/system/remove_proxy",
            web::delete().to(system::remove_proxy),
        )
        .route("/system/list_proxies", web::get().to(system::list_proxies))
        .service(
            Files::new("", &App::get_app().src)
                .index_file("index.html")
                .default_handler(fn_service(|req: ServiceRequest| async {
                    let (req, _) = req.into_parts();
                    let file = NamedFile::open_async(&format!("{}/index.html", App::get_app().src))
                        .await?;
                    let res = file.into_response(&req);
                    Ok(ServiceResponse::new(req, res))
                })),
        );
}

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Proxy;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S> Transform<S, ServiceRequest> for Proxy
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = ProxyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ProxyMiddleware { service }))
    }
}

pub struct ProxyMiddleware<S> {
    service: S,
}

impl<S> ProxyMiddleware<S> {
    async fn proxy(
        req: HttpRequest,
        mut payload: Payload,
        url: String,
    ) -> Result<ServiceResponse<BoxBody>, Error> {
        let method = req.method().clone();

        let headers = {
            let mut headers = reqwest::header::HeaderMap::new();
            for (name, value) in req.headers() {
                headers.insert(name.clone(), value.clone());
            }
            headers
        };

        let url = {
            let query = req.query_string();
            if query.is_empty() {
                url
            } else {
                format!("{url}?{query}")
            }
        };

        let body: bytes::Bytes = {
            let mut body = bytes::BytesMut::new();
            while let Ok(item) = payload.try_next().await {
                if let Some(bytes) = item {
                    body.extend(bytes);
                } else {
                    break;
                }
            }
            body.into()
        };

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        log::info!("proxy: {} {url}", method.as_str());

        match client
            .request(method, url)
            .headers(headers)
            .body(body)
            .send()
            .await
        {
            Ok(res) => {
                let status = res.status();
                let headers = {
                    let mut headers = HeaderMap::new();
                    for (name, value) in res.headers() {
                        headers.insert(name.clone(), value.clone());
                    }
                    headers
                };
                let body = res.bytes().await.unwrap();
                let mut res = HttpResponse::new(status);
                for (name, value) in headers {
                    res.headers_mut().insert(name.clone(), value.clone());
                }
                res = res.set_body(BoxBody::new(body));
                Ok(ServiceResponse::new(req, res))
            }
            Err(e) => {
                log::error!("{:?}", e);
                Ok(ServiceResponse::new(
                    req,
                    HttpResponse::new(StatusCode::NOT_FOUND),
                ))
            }
        }
    }
}

impl<S> Service<ServiceRequest> for ProxyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path();
        let proxies = App::get_app().proxies.lock().unwrap();
        for (name, url) in &*proxies {
            if path.starts_with(name) {
                let url = format!("{url}{}", &path[name.len()..]);
                let (req, payload) = req.into_parts();
                return Box::pin(Self::proxy(req, payload, url));
            }
        }
        drop(proxies);
        Box::pin(self.service.call(req))
    }
}
