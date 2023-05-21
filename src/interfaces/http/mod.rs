use actix_files::NamedFile;
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    http::{self, header::HeaderMap},
    web::{self, Bytes},
    HttpRequest, Responder,
};
use reqwest::StatusCode;

use crate::App;

pub(crate) fn config(cfg: &mut web::ServiceConfig) {
    let app = App::get_app();
    let path = app.config.path.clone();
    let src = app.config.src.clone();
    let service = app.config.service.clone();
    cfg.service(
        web::scope(&format!("{service}/{{domain}}"))
            .default_service(web::method(http::Method::GET).to(proxy))
            .default_service(web::method(http::Method::POST).to(proxy))
            .default_service(web::method(http::Method::PUT).to(proxy))
            .default_service(web::method(http::Method::DELETE).to(proxy))
            .default_service(web::method(http::Method::HEAD).to(proxy))
            .default_service(web::method(http::Method::OPTIONS).to(proxy))
            .default_service(web::method(http::Method::CONNECT).to(proxy))
            .default_service(web::method(http::Method::PATCH).to(proxy))
            .default_service(web::method(http::Method::TRACE).to(proxy)),
    )
    .service(
        web::scope(&path).service(
            actix_files::Files::new("", &src)
                .index_file("index.html")
                .default_handler(fn_service(|req: ServiceRequest| async {
                    let (req, _) = req.into_parts();
                    let file =
                        NamedFile::open_async(&format!("{}/index.html", App::get_config().src))
                            .await?;
                    let res = file.into_response(&req);
                    Ok(ServiceResponse::new(req, res))
                })),
        ),
    );
}

async fn proxy(
    req: HttpRequest,
    domain: web::Path<String>,
    body: Bytes,
) -> impl Responder {
    let method = req.method().clone();
    let url = {
        let service = &App::get_config().service;
        let req_path = req.path().replace(&format!("{service}/"), "");
        let path = &req_path[req_path.find('/').unwrap()..];
        let query = req.query_string();
        if query.is_empty() {
            format!("{}{path}", domain.as_str())
        } else {
            format!("{}{path}?{}", domain.as_str(), query)
        }
    };
    let headers = {
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in req.headers() {
            headers.insert(name.clone(), value.clone());
        }
        // let uri: Uri = domain.as_str().parse().unwrap();
        // headers.insert(reqwest::header::HOST, uri.host().unwrap().parse().unwrap());
        // headers.insert(reqwest::header::REFERER, domain.as_str().parse().unwrap());
        headers
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
            let mut res = body.customize().with_status(status);
            for header in headers {
                res = res.insert_header(header);
            }
            res
        }
        Err(e) => {
            log::error!("{:?}", e);
            Bytes::new().customize().with_status(StatusCode::NOT_FOUND)
        }
    }
}
