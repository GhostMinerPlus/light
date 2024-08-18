use std::sync::Arc;

use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
};
use edge_lib::{data::AsDataManager, engine::EdgeEngine, util::Path};
use reqwest::StatusCode;

pub async fn respone(
    path: &str,
    fake_path: &str,
    dm: Arc<dyn AsDataManager>,
    req: ServiceRequest,
    proxy: &str,
    edge_engine: &mut EdgeEngine,
) -> ServiceResponse<BoxBody> {
    let tail_path = &path[fake_path.len()..];
    let (req, payload) = req.into_parts();
    let name_v = dm
        .get(&Path::from_str(&format!("{proxy}->name")))
        .await
        .unwrap();
    let req_cell = inner::extract_req(&req, payload).await;
    if let Some(uri) = inner::get_uri_from_cache(edge_engine, &name_v[0])
        .await
        .unwrap()
    {
        let res = inner::proxy_fn(req_cell.clone(), format!("{uri}{tail_path}")).await;
        match res.status() {
            StatusCode::NOT_FOUND => (),
            _ => {
                return ServiceResponse::new(req, res);
            }
        }
    }
    let uri = inner::get_uri_from_remote(edge_engine, dm, &name_v[0])
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
    dm: Arc<dyn AsDataManager>,
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
    use edge_lib::{
        data::AsDataManager,
        engine::{EdgeEngine, ScriptTree},
        util::Path,
    };
    use futures_util::TryStreamExt;
    use reqwest::{header::HeaderValue, Method, StatusCode};
    use std::sync::Arc;

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
        edge_engine: &mut EdgeEngine,
        name: &str,
    ) -> err::Result<Option<String>> {
        let cache_rs = edge_engine
            .execute1(&ScriptTree {
                script: [format!("$->$:output inner root->web_server {name}<-name")].join("\n"),
                name: format!("web_server"),
                next_v: vec![ScriptTree {
                    script: [
                        format!("$->$:output = $->$:input->ip _"),
                        format!("$->$:output append $->$:output $->$:input->port"),
                        format!("$->$:output append $->$:output $->$:input->path"),
                    ]
                    .join("\n"),
                    name: format!("info"),
                    next_v: vec![],
                }],
            })
            .await
            .map_err(|e| err::Error::Other(e.to_string()))?;
        if !cache_rs["web_server"].is_empty() && cache_rs["web_server"]["info"].len() == 3 {
            let (ip, port, path) = parser::parse_info(&cache_rs)?;
            return Ok(Some(parser::parse_uri(ip, port, path)));
        }
        Ok(None)
    }

    pub async fn get_uri_from_remote(
        edge_engine: &mut EdgeEngine,
        dm: Arc<dyn AsDataManager>,
        name: &str,
    ) -> err::Result<String> {
        let moon_server_v = dm
            .get(&Path::from_str("root->moon_server"))
            .await
            .map_err(err::map_io_err)?;
        if moon_server_v.is_empty() {
            return Err(err::Error::Other(format!("no moon_server")));
        }
        let script_tree = ScriptTree {
            script: [format!("$->$:output inner root->web_server {name}<-name")].join("\n"),
            name: format!("web_server"),
            next_v: vec![ScriptTree {
                script: [
                    format!("$->$:output = $->$:input->ip _"),
                    format!("$->$:output append $->$:output $->$:input->port"),
                    format!("$->$:output append $->$:output $->$:input->path"),
                ]
                .join("\n"),
                name: format!("info"),
                next_v: vec![],
            }],
        };
        for moon_server in &moon_server_v {
            let rs = util::http_execute1(moon_server, &script_tree)
                .await
                .map_err(err::map_io_err)?;
            let rs = json::parse(&rs).map_err(|e| err::Error::Other(e.to_string()))?;
            log::debug!("web_servers: {rs}");
            let (ip, port, path) = parser::parse_info(&rs)?;
            if let Err(e) = edge_engine
                .execute1(&ScriptTree {
                    script: [
                        &format!("$->$:web_server = root->web_server {name}<-name"),
                        "$->$:web_server if $->$:web_server ?",
                        &format!("$->$:web_server->name = {name} _"),
                        &format!("$->$:web_server->ip = {ip} _"),
                        &format!("$->$:web_server->port = {port} _"),
                        &format!("$->$:web_server->path = {path} _"),
                        "root->web_server distinct root->web_server $->$:web_server",
                    ]
                    .join("\n"),
                    name: format!("result"),
                    next_v: vec![],
                })
                .await
            {
                log::warn!("failed to execute1 cache, caused by {e}\nwhen get_uri_by_name");
            } else if let Err(e) = edge_engine.commit().await {
                log::warn!("failed to commit cache, caused by {e}\nwhen get_uri_by_name");
            }
            return Ok(parser::parse_uri(ip, port, path));
        }
        Err(err::Error::Other(format!("no uri")))
    }

    mod parser {
        use crate::err;

        pub fn parse_info(rs: &json::JsonValue) -> err::Result<(&str, &str, &str)> {
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
            Ok((ip, port, path))
        }

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
