use actix_http::{body::BoxBody, Payload};
use actix_web::{
    dev::{forward_ready, Service, Transform},
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    web, Error, HttpRequest, HttpResponse,
};
use edge_lib::{data::AsDataManager, util::Path, EdgeEngine, ScriptTree};
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use reqwest::{header::HeaderValue, StatusCode};
use std::{
    future::{self, Ready},
    sync::Arc,
};

use crate::{err, util};

async fn proxy_fn(
    req: HttpRequest,
    token: &str,
    mut payload: Payload,
    uri: String,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let method = req.method().clone();

    let headers = {
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in req.headers() {
            if name == "Cookie" {
                headers.insert(
                    name.clone(),
                    HeaderValue::from_str(&format!("{};printer={token}", value.to_str().unwrap()))
                        .unwrap(),
                );
            } else {
                headers.insert(name.clone(), value.clone());
            }
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

async fn get_uri_by_name(dm: &dyn AsDataManager, name: &str) -> err::Result<String> {
    log::debug!("get_uri_by_name: {name}");
    let moon_server_v = dm
        .get(&Path::from_str("root->moon_server"))
        .await
        .map_err(err::map_io_err)?;
    let script_tree = ScriptTree {
        script: [format!("$->$output = inner root->web_server {name}<-name")].join("\n"),
        name: format!("web_server"),
        next_v: vec![ScriptTree {
            script: [
                format!("$->$output = = $->$input->ip _"),
                format!("$->$output += = $->$input->port _"),
                format!("$->$output += = $->$input->path _"),
            ]
            .join("\n"),
            name: format!("info"),
            next_v: vec![],
        }],
    };
    let script_str = EdgeEngine::tree_2_entry(&script_tree).to_string();
    for moon_server in &moon_server_v {
        let rs = util::http_execute(moon_server, script_str)
            .await
            .map_err(err::map_io_err)?;
        let rs = json::parse(&rs).map_err(|e| err::Error::Other(e.to_string()))?;
        log::debug!("web_servers: {rs}");
        let info = &rs["web_server"]["info"][0];
        let ip = info[0]
            .as_str()
            .ok_or(err::Error::Other(format!("no ip")))?;
        let port = info[1]
            .as_str()
            .ok_or(err::Error::Other(format!("no port")))?;
        let path = info[2]
            .as_str()
            .ok_or(err::Error::Other(format!("no path")))?;
        let uri = if ip.contains(':') {
            if port == "80" {
                format!("http://[{ip}]{path}")
            } else {
                format!("http://[{ip}]:{port}{path}")
            }
        } else {
            if port == "80" {
                format!("http://{ip}{path}")
            } else {
                format!("http://{ip}:{port}{path}")
            }
        };
        return Ok(uri);
    }
    Err(err::Error::Other(format!("no uri")))
}

// Public
pub struct ProxyMiddleware<S> {
    service: Arc<S>,
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
        let service = self.service.clone();
        Box::pin(async move {
            let path = req.path().to_string();
            log::info!("request: {path}");
            let dm = req
                .app_data::<web::Data<Arc<dyn AsDataManager>>>()
                .unwrap()
                .as_ref()
                .clone();
            let proxy_v = dm.get(&Path::from_str("root->proxy")).await.unwrap();
            for proxy in &proxy_v {
                let fake_path_v = dm
                    .get(&Path::from_str(&format!("{proxy}->path")))
                    .await
                    .unwrap();
                if path.starts_with(&fake_path_v[0]) {
                    let tail_path = &path[fake_path_v[0].len()..];
                    let (req, payload) = req.into_parts();
                    let name_v = dm
                        .get(&Path::from_str(&format!("{proxy}->name")))
                        .await
                        .unwrap();
                    let uri = get_uri_by_name(&*dm, &name_v[0]).await.unwrap();
                    let token_v = dm.get(&Path::from_str("root->token")).await.unwrap();
                    return proxy_fn(
                        req,
                        token_v.first().unwrap(),
                        payload,
                        format!("{uri}{tail_path}"),
                    )
                    .await;
                }
            }
            service.call(req).await
        })
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
        future::ready(Ok(ProxyMiddleware {
            service: Arc::new(service),
        }))
    }
}
