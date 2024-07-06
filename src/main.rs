//! Start server

use std::{collections::BTreeMap, io, sync::Arc};

use earth::AsConfig;
use edge_lib::{
    data::{Auth, MemDataManager, RecDataManager},
    EdgeEngine, ScriptTree,
};
use light::{connector, server};

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
    domain: String,
    key: String,
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
            domain: format!("_"),
            key: String::new(),
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
            let global = Arc::new(MemDataManager::new(Auth::printer(&config.name)));
            let dm = Arc::new(RecDataManager::new(global));
            let mut edge_engine = EdgeEngine::new(dm.clone());
            // config.ip, config.port, config.name
            let token = light::util::gen_token(&config.key, config.name.clone(), None).unwrap();
            edge_engine
                .execute1(&ScriptTree {
                    script: [
                        format!("root->name = {} _", config.name),
                        format!("root->ip = {} _", config.ip),
                        format!("root->port = {} _", config.port),
                        format!("root->path = {} _", config.path),
                        format!("root->src = {} _", config.src),
                        format!("root->domain = {} _", config.domain),
                        format!("root->token = {} _", token),
                    ]
                    .join("\n"),
                    name: format!("result"),
                    next_v: vec![],
                })
                .await?;
            let option_script = config
                .moon_servers
                .iter()
                .map(|moon_server| {
                    format!("root->moon_server append root->moon_server {moon_server}")
                })
                .reduce(|acc, line| format!("{acc}\n{line}"));
            if let Some(script) = option_script {
                edge_engine
                    .execute1(&ScriptTree {
                        script,
                        name: format!("result"),
                        next_v: vec![],
                    })
                    .await?;
            }
            let option_script1 = config
                .proxy
                .into_iter()
                .map(|(path, name)| {
                    [
                        "$->$proxy = ? _",
                        &format!("$->$proxy->path = {path} _"),
                        &format!("$->$proxy->name = {name} _"),
                        "root->proxy append root->proxy $->$proxy",
                    ]
                    .join("\n")
                })
                .reduce(|acc, block| format!("{acc}\n{block}"));
            if let Some(script) = option_script1 {
                edge_engine
                    .execute1(&ScriptTree {
                        script,
                        name: format!("result"),
                        next_v: vec![],
                    })
                    .await?;
            }
            edge_engine.commit().await?;

            tokio::spawn(connector::HttpConnector::new(dm.clone()).run());
            server::WebServer::new(dm).run().await
        })
}
