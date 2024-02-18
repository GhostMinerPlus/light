//! Start server

use std::{collections::BTreeMap, io};

use earth::AsConfig;

mod server;
mod star;

// Public
#[derive(serde::Deserialize, serde::Serialize, AsConfig, Clone)]
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
    hosts: Vec<String>,
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
            hosts: Vec::new(),
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
    // Run server
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.thread_num as usize)
        .enable_all()
        .build()?
        .block_on(
            server::Server::new(
                config.ip,
                config.port,
                config.path,
                config.name,
                config.src,
                config.hosts,
                config.proxy,
                config.moon_servers
            )
            .run(),
        )
}
