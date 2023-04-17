use actix_files::NamedFile;
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    http::{self, header::HeaderMap, Uri},
    web::{self, Bytes},
    HttpRequest, HttpServer, Responder,
};
use reqwest::StatusCode;

static mut APP: Option<App> = None;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    name: String,
    domain: String,
    path: String,
    service: String,
    src: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "Light".to_string(),
            domain: "[::]:8080".to_string(),
            path: "".to_string(),
            service: "/service".to_string(),
            src: ".".to_string(),
        }
    }
}

impl earth::Config for Config {}

pub struct App {
    config: Config,
}

impl App {
    pub fn create_app(config: Config) -> &'static mut App {
        unsafe {
            APP = Some(App { config });
            APP.as_mut().unwrap()
        }
    }

    pub fn run(&mut self) {
        let domain = self.config.domain.clone();
        let path = self.config.path.clone();
        let src = self.config.src.clone();
        let service = self.config.service.clone();
        log::info!(
            "{} serving at: http://{domain}{path} from {src}",
            self.config.name
        );
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .service(
                    web::scope(&format!("{service}/{{domain}}"))
                        .default_service(web::method(http::Method::GET).to(Self::service))
                        .default_service(web::method(http::Method::POST).to(Self::service))
                        .default_service(web::method(http::Method::PUT).to(Self::service))
                        .default_service(web::method(http::Method::DELETE).to(Self::service))
                        .default_service(web::method(http::Method::HEAD).to(Self::service))
                        .default_service(web::method(http::Method::OPTIONS).to(Self::service))
                        .default_service(web::method(http::Method::CONNECT).to(Self::service))
                        .default_service(web::method(http::Method::PATCH).to(Self::service))
                        .default_service(web::method(http::Method::TRACE).to(Self::service)),
                )
                .service(
                    web::scope(&path).service(
                        actix_files::Files::new("", &src)
                            .index_file("index.html")
                            .default_handler(fn_service(|req: ServiceRequest| async {
                                let (req, _) = req.into_parts();
                                let file = NamedFile::open_async(&format!(
                                    "{}/index.html",
                                    Self::get_config().src
                                ))
                                .await?;
                                let res = file.into_response(&req);
                                Ok(ServiceResponse::new(req, res))
                            })),
                    ),
                )
        });
        let _ = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(
                match server.bind(&domain) {
                    Ok(server) => server,
                    Err(e) => {
                        log::error!("{:?}", e);
                        return;
                    }
                }
                .run(),
            );
    }

    fn get_app() -> &'static App {
        unsafe { APP.as_ref().unwrap() }
    }

    fn get_config() -> &'static Config {
        &Self::get_app().config
    }

    async fn service(
        req: HttpRequest,
        domain: actix_web::web::Path<String>,
        body: Bytes,
    ) -> impl Responder {
        let method = req.method().clone();
        let url = {
            let service = &Self::get_config().service;
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
            let uri: Uri = domain.as_str().parse().unwrap();
            headers.insert(reqwest::header::HOST, uri.host().unwrap().parse().unwrap());
            headers.insert(reqwest::header::REFERER, domain.as_str().parse().unwrap());
            headers
        };
        match reqwest::Client::new()
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
}
