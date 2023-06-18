mod interfaces;

use std::collections::BTreeMap;

use actix_web::HttpServer;
use env_logger::Env;

use crate::interfaces::Proxy;

static mut APP: Option<App> = None;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut config = Config::default();
    earth::Config::merge_by_file(&mut config, "earth.toml");
    earth::Config::merge_by_args(&mut config, &std::env::args().collect());

    App::create_app(config).run();
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct Config {
    name: String,
    domain: String,
    path: String,
    src: String,

    proxies: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "Light".to_string(),
            domain: "[::]:8080".to_string(),
            path: "/light".to_string(),
            src: ".".to_string(),
            proxies: BTreeMap::new(),
        }
    }
}

impl earth::Config for Config {}

pub(crate) struct App {
    config: Config,
}

impl App {
    pub(crate) fn get_app() -> &'static App {
        unsafe { APP.as_ref().unwrap() }
    }

    pub(crate) fn get_config() -> &'static Config {
        &Self::get_app().config
    }

    fn create_app(config: Config) -> &'static mut App {
        unsafe {
            APP = Some(App { config });
            APP.as_mut().unwrap()
        }
    }

    fn run(&mut self) {
        self.start_http_service();
    }

    fn start_http_service(&self) {
        log::info!("start http service");
        let domain = Self::get_app().config.domain.clone();
        let path = Self::get_app().config.path.clone();
        log::info!("http service serves at: http://{domain}{path}");
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .wrap(Proxy)
                .service(actix_web::web::scope(&path).configure(interfaces::http::config))
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
}
