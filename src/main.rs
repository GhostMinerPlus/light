//! Start server

use std::{collections::BTreeMap, io, sync::Arc, time::Duration};

use earth::AsConfig;
use edge_lib::{data::DataManager, mem_table, AsEdgeEngine, EdgeEngine};
use tokio::{sync::Mutex, time};

mod server;
mod star;

// Public
#[derive(serde::Deserialize, serde::Serialize, AsConfig, Clone, Debug)]
/// Config
struct Config {
    /// Default: light
    name: String,
    /// Default: 0.0.0.0
    ip: String,
    /// Default: 80
    port: u16,
    /// Default: light
    path: String,
    proxy: BTreeMap<String, String>,
    /// Default: info
    log_level: String,
    /// Default: dist
    src: String,
    /// Default: 8
    thread_num: u8,
    moon_servers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "light".to_string(),
            ip: "0.0.0.0".to_string(),
            port: 80,
            path: "/light".to_string(),
            proxy: BTreeMap::new(),
            log_level: "info".to_string(),
            src: "dist".to_string(),
            thread_num: 8,
            moon_servers: Vec::new(),
        }
    }
}

fn main() -> io::Result<()> {
    // Parse config
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
        config.merge_by_arg_v(&arg_v);
    }
    config.merge_by_env(&format!("{}", config.name));
    // Config log
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();
    log::debug!("{:?}", config);
    // Run server
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.thread_num as usize)
        .enable_all()
        .build()?
        .block_on(async {
            let global = Arc::new(Mutex::new(mem_table::MemTable::new()));
            let mut edge_engine = EdgeEngine::new(DataManager::with_global(global.clone()));
            // config.ip, config.port, config.name
            let script = [
                format!("PktUdpServer->name = = {} _", config.name),
                format!("PktUdpServer->ip = = {} _", config.ip),
                format!("PktUdpServer->port = = {} _", config.port),
                format!("HttpServer->name = = {} _", config.name),
                format!("HttpServer->ip = = {} _", config.ip),
                format!("HttpServer->port = = {} _", config.port),
            ]
            .join("\\n");
            edge_engine
                .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
                .await?;
            edge_engine.commit().await?;

            tokio::spawn(async move {
                loop {
                    log::info!("alive");
                    time::sleep(Duration::from_secs(10)).await;
                    if let Err(e) = star::report_uri(
                        &config.name,
                        config.port,
                        &config.path,
                        &config.moon_servers,
                    )
                    .await
                    {
                        log::error!("{e}");
                    }
                }
            });

            // server::WebServer::new(global).run().await
            loop {
                log::info!("alive");
                time::sleep(Duration::from_secs(10)).await;
            }
        })
}
