use actix_http::{body::BoxBody, Payload};
use actix_web::{
    dev::{forward_ready, Service, Transform},
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    web, Error, HttpRequest, HttpResponse,
};
use edge_lib::mem_table::MemTable;
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use reqwest::StatusCode;
use std::{
    future::{self, Ready},
    sync::{Arc, Mutex},
};

async fn proxy_fn(
    req: HttpRequest,
    mut payload: Payload,
    uri: String,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let method = req.method().clone();

    let headers = {
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in req.headers() {
            headers.insert(name.clone(), value.clone());
        }
        headers
    };

    let uri = {
        let query = req.query_string();
        if query.is_empty() {
            uri
        } else {
            format!("{uri}?{query}")
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
    log::info!("proxy: {} {uri}", method.as_str());

    match client
        .request(method, uri)
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

// Public
pub struct ProxyMiddleware<S> {
    service: S,
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
        let global = req
            .app_data::<web::Data<Arc<Mutex<MemTable>>>>()
            .unwrap()
            .clone();
        let mut dm = global.lock().unwrap();
        let proxy_v = dm.get_target_v_unchecked("root", "proxy");
        let path = req.path();
        log::info!("request: {path}");
        for proxy in &proxy_v {
            let fake_path = dm.get_target(proxy, "name").unwrap();
            let uri = dm.get_target(proxy, "uri").unwrap();
            if path.starts_with(&fake_path) {
                let tail_path = path[fake_path.len()..].to_string();
                let (req, payload) = req.into_parts();
                return Box::pin(proxy_fn(req, payload, format!("{uri}{tail_path}")));
            }
        }
        Box::pin(self.service.call(req))
    }
}

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Proxy {}

impl Proxy {
    pub fn new() -> Self {
        Self {}
    }
}

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
