use std::{collections::BTreeMap, sync::Mutex};

use config::Config;

use crate::interfaces::{self, http};

static mut CONTEXT: Option<Context> = None;

// public
pub mod config;

pub fn run() {
    // config
    let mut config = Config::default();
    earth::Config::merge_by_file(&mut config, "earth.toml");
    earth::Config::merge_by_args(&mut config, &std::env::args().collect());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // init
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or(&config.log_level),
            )
            .init();
            Context::init(&config).await;
            http::init(&config).await;

            // run
            interfaces::run().await;
        });
}

pub struct Context {
    pub name: String,
    pub domain: String,
    pub path: String,

    pub src: String,

    pub proxies: Mutex<BTreeMap<String, String>>,
}

impl Context {
    pub async fn init(config: &Config) {
        unsafe {
            CONTEXT = Some(Context {
                domain: config.domain.clone(),
                path: config.path.clone(),
                name: config.name.clone(),
                src: config.src.clone(),
                proxies: Mutex::new(BTreeMap::new()),
            });
        };
    }

    pub fn as_ref() -> &'static Context {
        unsafe { CONTEXT.as_ref().unwrap() }
    }
}
