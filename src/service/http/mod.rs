pub mod service;

use actix_files::{Files, NamedFile};
use actix_http::{body::BoxBody, Payload};
use actix_web::{
    dev::{forward_ready, Service, Transform},
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    Error, HttpRequest, HttpResponse, HttpServer, web,
};
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use reqwest::StatusCode;
use std::{
    future::{self, Ready},
    io,
};

use crate::util::Context;

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
        let ctx = req.app_data::<web::Data<Context>>().unwrap().clone();
        let path = req.path();
        log::info!("request: {path}");
        let proxies = ctx.proxy.lock().unwrap();
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

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
struct Proxy {}

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
        future::ready(Ok(ProxyMiddleware { service }))
    }
}

fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path)
        .service(service::system::add_proxy)
        .service(service::system::remove_proxy)
        .service(service::system::list_proxies)
        .service(
            Files::new("", &src)
                .index_file("index.html")
                .default_handler(actix_web::dev::fn_service(move |req: ServiceRequest| {
                    let index_html = format!("{}/index.html", src);
                    let (req, _) = req.into_parts();
                    async {
                        let file = NamedFile::open_async(index_html).await?;
                        let res = file.into_response(&req);
                        Ok(ServiceResponse::new(req, res))
                    }
                })),
        )
}

// public
pub async fn init(domain: &str, path: &str, hosts: &Vec<String>) -> io::Result<()> {
    let client = reqwest::Client::new();
    let proxy = serde_json::to_string(&service::dto::Proxy {
        path: path.to_string(),
        url: format!("http://{}{}", domain, path),
    })?;
    for host in hosts {
        client
            .post(format!("{host}/system/add_proxy"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(proxy.clone())
            .send()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    }
    Ok(())
}

pub async fn run(ctx: Context) -> io::Result<()> {
    let domain = ctx.domain.clone();
    let path = ctx.path.clone();
    let src = ctx.src.clone();
    log::info!("http service uri: http://{domain}{path}");

    let server = HttpServer::new(move || {
        actix_web::App::new()
            .app_data(web::Data::new(ctx.clone()))
            .wrap(Proxy {})
            .service(config(&path, &src))
    });
    server.bind(&domain)?.run().await
}
