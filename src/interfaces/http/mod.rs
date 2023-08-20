pub mod service;

use actix_files::{Files, NamedFile};
use actix_http::{body::BoxBody, Payload};
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    dev::{forward_ready, Service, Transform},
    http::header::HeaderMap,
    web, Error, HttpRequest, HttpResponse, HttpServer,
};
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use reqwest::StatusCode;
use std::future::{ready, Ready};

use crate::{
    application::dto,
    infrastructure::{config::Config, Context},
};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
struct Proxy;

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

struct ProxyMiddleware<S> {
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
        let proxies = Context::as_ref().proxy.lock().unwrap();
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

fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(service::system::add_proxy)
        .service(service::system::remove_proxy)
        .service(service::system::list_proxies)
        .service(
            Files::new("", &Context::as_ref().src)
                .index_file("index.html")
                .default_handler(fn_service(|req: ServiceRequest| async {
                    let (req, _) = req.into_parts();
                    let file =
                        NamedFile::open_async(&format!("{}/index.html", Context::as_ref().src))
                            .await?;
                    let res = file.into_response(&req);
                    Ok(ServiceResponse::new(req, res))
                })),
        );
}

// public
pub async fn init(config: &Config) {
    let client = reqwest::Client::new();
    let proxy = serde_json::to_string(&dto::Proxy {
        path: config.path.clone(),
        url: format!("http://{}{}", config.domain, config.path),
    })
    .unwrap();
    for host in &config.hosts {
        client
            .post(format!("{host}/system/add_proxy"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(proxy.clone())
            .send()
            .await
            .unwrap();
    }
}

pub async fn run() {
    let context = Context::as_ref();
    let path = context.path.clone();
    let domain = context.domain.clone();
    log::info!("http service uri: http://{domain}{path}");

    let server = HttpServer::new(move || {
        actix_web::App::new().service(actix_web::web::scope(&path).configure(config))
    });
    server.bind(&domain).unwrap().run().await.unwrap();
}
