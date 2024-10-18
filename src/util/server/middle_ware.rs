use actix_http::body::BoxBody;
use actix_web::{
    dev::{forward_ready, Service, Transform},
    dev::{ServiceRequest, ServiceResponse},
    web, Error,
};
use edge_lib::util::{
    data::{AsDataManager, MemDataManager},
    Path,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{self, Ready},
    sync::Arc,
};
use tokio::sync::Mutex;

mod proxy;

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
            let global_mutex = req
                .app_data::<web::Data<Arc<Mutex<MemDataManager>>>>()
                .unwrap()
                .as_ref()
                .clone();
            let mut global = global_mutex.lock().await;

            if path.starts_with(proxy::MOON_SERVICE_PATH) {
                return Ok(proxy::respone_moon(&path, &mut *global, req).await);
            }
            let proxy_v = global.get(&Path::from_str("root->proxy")).await.unwrap();
            for proxy in &proxy_v {
                let fake_path_v = global
                    .get(&Path::from_str(&format!("{proxy}->path")))
                    .await
                    .unwrap();
                if path.starts_with(&fake_path_v[0]) {
                    return Ok(
                        proxy::respone(&path, &fake_path_v[0], &mut *global, req, proxy).await,
                    );
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
