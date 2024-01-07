mod service;
mod util;

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

use earth::AsConfig;

// public
#[derive(serde::Deserialize, serde::Serialize, AsConfig, Clone)]
struct Config {
    name: String,
    ip: String,
    port: u16,
    path: String,
    hosts: Vec<String>,
    proxy: BTreeMap<String, String>,
    log_level: String,
    src: String,
    thread_num: u8,
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
            thread_num: 8,
        }
    }
}

fn main() -> io::Result<()> {
    // config
    let mut config = Config::default();
    let mut arg_v: Vec<String> = std::env::args().collect();
    arg_v.remove(0);
    let file_name = if !arg_v.is_empty() && !arg_v[0].starts_with("--") {
        arg_v.remove(0)
    } else {
        "config.toml".to_string()
    };
    config.merge_by_file(&file_name);
    if !arg_v.is_empty() {
        config.merge_by_args(&arg_v);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.thread_num as usize)
        .enable_all()
        .build()?
        .block_on(async {
            start_task(&config).await?;
            serve(&config).await
        })
}

async fn start_task(_: &Config) -> io::Result<()> {
    Ok(())
}

async fn serve(config: &Config) -> io::Result<()> {
    let domain = format!("{}:{}", config.ip, config.port);
    service::init(&domain, &config.path, &config.hosts).await?;

    let ctx = util::Context {
        domain,
        path: config.path.clone(),
        name: config.name.clone(),
        src: config.src.clone(),
        proxy: Arc::new(Mutex::new(config.proxy.clone())),
    };
    log::info!("{} starting", ctx.name);
    service::run(ctx).await
}
