use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
};
use edge_lib::util::{
    data::{AsDataManager, MemDataManager},
    Path,
};
use reqwest::StatusCode;

pub async fn respone(
    path: &str,
    fake_path: &str,
    global: &mut MemDataManager,
    req: ServiceRequest,
    proxy: &str,
) -> ServiceResponse<BoxBody> {
    let tail_path = &path[fake_path.len()..];
    let (req, payload) = req.into_parts();
    let name_v = global
        .get(&Path::from_str(&format!("{proxy}->name")))
        .await
        .unwrap();
    let req_cell = inner::extract_req(&req, payload).await;
    if let Some(uri) = inner::get_uri_from_cache(global, &name_v[0]).await.unwrap() {
        let res = inner::proxy_fn(req_cell.clone(), format!("{uri}{tail_path}")).await;
        match res.status() {
            StatusCode::NOT_FOUND => (),
            _ => {
                return ServiceResponse::new(req, res);
            }
        }
    }
    let uri = inner::get_uri_from_remote(global, &name_v[0])
        .await
        .unwrap();
    return ServiceResponse::new(
        req,
        inner::proxy_fn(req_cell, format!("{uri}{tail_path}")).await,
    );
}

pub const MOON_SERVICE_PATH: &str = "/moon_server";

pub async fn respone_moon(
    path: &str,
    dm: &mut MemDataManager,
    req: ServiceRequest,
) -> ServiceResponse<BoxBody> {
    let (req, payload) = req.into_parts();
    let req_cell = inner::extract_req(&req, payload).await;
    let moon_server_v = dm.get(&Path::from_str("root->moon_server")).await.unwrap();
    let uri = &moon_server_v[0];
    let tail_path = &path[MOON_SERVICE_PATH.len()..];
    return ServiceResponse::new(
        req,
        inner::proxy_fn(req_cell, format!("{uri}{tail_path}")).await,
    );
}

mod inner {
    use actix_http::{body::BoxBody, Payload};
    use actix_web::{http::header::HeaderMap, HttpRequest, HttpResponse};
    use edge_lib::util::{
        data::{AsDataManager, MemDataManager},
        engine::{AsEdgeEngine, EdgeEngine},
        rs_2_str, Path,
    };
    use futures_util::TryStreamExt;
    use reqwest::{header::HeaderValue, Method, StatusCode};

    use crate::{err, util};

    pub async fn extract_req(
        req: &HttpRequest,
        mut payload: Payload,
    ) -> (Method, reqwest::header::HeaderMap, String, bytes::Bytes) {
        (
            req.method().clone(),
            {
                let mut headers = reqwest::header::HeaderMap::new();
                for (name, value) in req.headers() {
                    if name == "Cookie" {
                        headers.insert(
                            name.clone(),
                            HeaderValue::from_str(&format!("{}", value.to_str().unwrap())).unwrap(),
                        );
                    } else {
                        headers.insert(name.clone(), value.clone());
                    }
                }
                headers
            },
            req.query_string().to_string(),
            {
                let mut body = bytes::BytesMut::new();
                while let Ok(item) = payload.try_next().await {
                    if let Some(bytes) = item {
                        body.extend(bytes);
                    } else {
                        break;
                    }
                }
                body.into()
            },
        )
    }

    pub async fn proxy_fn(
        req: (Method, reqwest::header::HeaderMap, String, bytes::Bytes),
        uri: String,
    ) -> HttpResponse {
        let uri = {
            let query = &req.2;
            if query.is_empty() {
                uri
            } else {
                format!("{uri}?{query}")
            }
        };

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        log::info!("proxy: {} {uri}", req.0.as_str());

        match client
            .request(req.0, uri)
            .headers(req.1)
            .body(req.3)
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
                res
            }
            Err(e) => {
                log::error!("{:?}", e);
                HttpResponse::new(StatusCode::NOT_FOUND)
            }
        }
    }

    pub async fn get_uri_from_cache(
        global: &mut MemDataManager,
        name: &str,
    ) -> err::Result<Option<String>> {
        let mut edge_engine = EdgeEngine::new(global);
        let web_server_v = json::parse(&rs_2_str(
            &edge_engine
                .execute_script(&[
                    format!("$->$:web_server inner root->web_server {name}<-name"),
                    format!("$->$:web_server->$:ip = $->$:web_server->ip _"),
                    format!("$->$:web_server->$:port = $->$:web_server->port _"),
                    format!("$->$:web_server->$:path = $->$:web_server->path _"),
                    format!("$->$:output dump $->$:web_server $"),
                ])
                .await
                .map_err(|e| err::Error::Other(e.to_string()))?,
        ))
        .unwrap();

        if web_server_v.len() == 1 {
            let (ip, port, path) = (
                web_server_v[0]["ip"][0].as_str().unwrap(),
                web_server_v[0]["port"][0].as_str().unwrap(),
                web_server_v[0]["path"][0].as_str().unwrap(),
            );
            return Ok(Some(parser::parse_uri(ip, port, path)));
        }
        Ok(None)
    }

    pub async fn get_uri_from_remote(
        global: &mut MemDataManager,
        name: &str,
    ) -> err::Result<String> {
        let moon_server_v = global
            .get(&Path::from_str("root->moon_server"))
            .await
            .map_err(err::map_io_err)?;
        if moon_server_v.is_empty() {
            return Err(err::Error::Other(format!("no moon_server")));
        }

        let mut edge_engine = EdgeEngine::new(global);

        for moon_server in &moon_server_v {
            let rs = util::native::http_execute_script(
                moon_server,
                &[
                    format!("$->$:web_server inner root->web_server {name}<-name"),
                    format!("$->$:web_server->$:ip = $->$:web_server->ip _"),
                    format!("$->$:web_server->$:port = $->$:web_server->port _"),
                    format!("$->$:web_server->$:path = $->$:web_server->path _"),
                    format!("$->$:output dump $->$:web_server $"),
                ],
            )
            .await
            .map_err(err::map_io_err)?;
            let web_server_v =
                json::parse(&rs_2_str(&rs)).map_err(|e| err::Error::Other(e.to_string()))?;
            log::debug!("web_servers: {web_server_v}");
            let (ip, port, path) = (
                web_server_v[0]["ip"][0].as_str().unwrap(),
                web_server_v[0]["port"][0].as_str().unwrap(),
                web_server_v[0]["path"][0].as_str().unwrap(),
            );
            if let Err(e) = edge_engine
                .execute_script(&[
                    format!("$->$:web_server = root->web_server {name}<-name"),
                    format!("$->$:web_server if $->$:web_server ?"),
                    format!("$->$:web_server->name = {name} _"),
                    format!("$->$:web_server->ip = {ip} _"),
                    format!("$->$:web_server->port = {port} _"),
                    format!("$->$:web_server->path = {path} _"),
                    format!("root->web_server distinct root->web_server $->$:web_server"),
                ])
                .await
            {
                log::warn!("failed to execute1 cache, caused by {e}\nwhen get_uri_by_name");
            }
            return Ok(parser::parse_uri(ip, port, path));
        }
        Err(err::Error::Other(format!("no uri")))
    }

    mod parser {
        pub fn parse_uri(ip: &str, port: &str, path: &str) -> String {
            if ip.contains(':') {
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
            }
        }
    }
}
