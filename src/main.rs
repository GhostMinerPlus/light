pub(crate) mod application;
pub(crate) mod interfaces;

use std::{collections::BTreeMap, sync::Mutex};

use actix_web::HttpServer;
use env_logger::Env;

static mut APP: Option<App> = None;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut config = Config::default();
    earth::Config::merge_by_file(&mut config, "earth.toml");
    earth::Config::merge_by_args(&mut config, &std::env::args().collect());
    App::start_app(config);
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct Config {
    name: String,
    domain: String,
    path: String,
    src: String,
    hosts: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "Light".to_string(),
            domain: "[::1]:8080".to_string(),
            path: "/light".to_string(),
            src: ".".to_string(),
            hosts: Vec::new(),
        }
    }
}

impl earth::Config for Config {}

pub(crate) struct App {
    pub(crate) name: String,
    pub(crate) domain: String,
    pub(crate) path: String,
    pub(crate) src: String,

    pub(crate) proxies: Mutex<BTreeMap<String, String>>,
}

impl App {
    pub(crate) fn get_app() -> &'static App {
        unsafe { APP.as_ref().unwrap() }
    }

    fn start_app(config: Config) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let this = unsafe {
                    APP = Some(App {
                        name: config.name,
                        domain: config.domain,
                        path: config.path,
                        src: config.src,
                        proxies: Mutex::new(BTreeMap::new()),
                    });
                    APP.as_mut().unwrap()
                };
                this.register_hosts(&config.hosts).await;
                this.start_http_service().await;
            });
    }

    async fn register_hosts(&self, hosts: &Vec<String>) {
        let client = reqwest::Client::new();
        let proxy = serde_json::to_string(&application::dto::Proxy {
            path: self.path.clone(),
            url: format!("http://{}{}", self.domain, self.path),
        })
        .unwrap();
        for host in hosts {
            client
                .post(format!("{host}/system/add_proxy"))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(proxy.clone())
                .send()
                .await
                .unwrap();
        }
    }

    async fn start_http_service(&self) {
        log::info!("start http service");
        let domain = self.domain.clone();
        let path = self.path.clone();
        log::info!("http service serves at: http://{domain}{path}");
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .wrap(interfaces::http::Proxy {})
                .service(actix_web::web::scope(&path).configure(interfaces::http::config))
        });

        server.bind(&domain).unwrap().run().await.unwrap();
    }
}
