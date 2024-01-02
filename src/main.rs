mod service;
mod task;
mod util;

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

// public
#[derive(serde::Deserialize, serde::Serialize, earth::Config, Clone)]
struct Config {
    name: String,
    ip: String,
    port: u16,
    path: String,
    hosts: Vec<String>,
    proxy: BTreeMap<String, String>,
    log_level: String,
    src: String,
    host_v: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "light".to_string(),
            ip: "[::]".to_string(),
            port: 8080,
            path: "/light".to_string(),
            hosts: Vec::new(),
            proxy: BTreeMap::new(),
            log_level: "info".to_string(),
            src: "dist".to_string(),
            host_v: Vec::new(),
        }
    }
}

fn main() -> io::Result<()> {
    // config
    let mut config = Config::default();
    let arg_v: Vec<String> = std::env::args().collect();
    let file_name = if arg_v.len() == 2 {
        arg_v[1].as_str()
    } else {
        "config.toml"
    };
    earth::Config::merge_by_file(&mut config, file_name);

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            start_task(&config).await?;
            serve(&config).await
        })
}

async fn start_task(config: &Config) -> io::Result<()> {
    let config_copy = config.clone();
    log::info!("starting task 'report_ipv6'");
    tokio::spawn(async move {
        loop {
            task::report_address6(&config_copy.name, config_copy.port, &config_copy.host_v).await;
        }
    });
    Ok(())
}

async fn serve(config: &Config) -> io::Result<()> {
    let domain = format!("{}:{}", config.ip, config.port);
    service::init(&domain, &config.path, &config.hosts).await?;

    let ctx = Arc::new(util::Context {
        domain,
        path: config.path.clone(),
        name: config.name.clone(),
        src: config.src.clone(),
        proxy: Mutex::new(config.proxy.clone()),
    });
    log::info!("{} starting", ctx.name);
    service::run(ctx).await
}
