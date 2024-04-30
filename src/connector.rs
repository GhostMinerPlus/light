use std::{io, sync::Arc, time::Duration};

use edge_lib::{data::DataManager, mem_table::MemTable, AsEdgeEngine, EdgeEngine};
use tokio::{sync::Mutex, time};

use crate::util::{self, http_execute};

pub struct HttpConnector {
    global: Arc<Mutex<MemTable>>,
}

impl HttpConnector {
    pub fn new(global: Arc<Mutex<MemTable>>) -> Self {
        Self { global }
    }

    pub async fn run(self) -> io::Result<()> {
        loop {
            let mut edge_engine = EdgeEngine::new(DataManager::with_global(self.global.clone()));

            let script = [
                "$->$output = = root->name _",
                "$->$output += = root->port _",
                "$->$output += = root->path _",
                "info",
            ]
            .join("\\n");
            let rs = edge_engine
                .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
                .await?;
            log::debug!("{rs}");
            let name = rs["info"][0].as_str().unwrap();
            let ip = util::native::get_global_ipv6()?;
            let port = rs["info"][1].as_str().unwrap();
            let path = rs["info"][2].as_str().unwrap().to_string();

            let script = ["$->$output = = HttpConnector->uri _", "info"].join("\\n");
            let rs = edge_engine
                .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
                .await?;
            log::debug!("{rs}");
            let uri_v = &rs["info"];

            let script = [
                &format!("$->$server_exists = inner root->web_server {name}<-name"),
                "$->$web_server = if $->$server_exists ?",
                &format!("$->$web_server->name = = {name} _"),
                &format!("$->$web_server->ip = = {ip} _"),
                &format!("$->$web_server->port = = {port} _"),
                &format!("$->$web_server->path = = {path} _"),
                "root->web_server += left $->$web_server $->$server_exists",
                "info",
            ]
            .join("\\n");
            for uri in uri_v.members() {
                let uri = uri.as_str().unwrap();
                http_execute(uri, script.clone()).await?;
            }

            time::sleep(Duration::from_secs(10)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use edge_lib::{data::DataManager, mem_table, AsEdgeEngine, EdgeEngine};
    use tokio::sync::Mutex;

    #[test]
    fn test() {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let global = Arc::new(Mutex::new(mem_table::MemTable::new()));
                let mut edge_engine = EdgeEngine::new(DataManager::with_global(global.clone()));
                // config.ip, config.port, config.name
                let name = "test";
                let ip = "0.0.0.0";
                let port = "8080";
                let path = "/test";
                let script = [
                    &format!("$->$server_exists = inner root->web_server {name}<-name"),
                    "$->$web_server = if $->$server_exists ?",
                    &format!("$->$web_server->name = = {name} _"),
                    &format!("$->$web_server->ip = = {ip} _"),
                    &format!("$->$web_server->port = = {port} _"),
                    &format!("$->$web_server->path = = {path} _"),
                    "root->web_server += left $->$web_server $->$server_exists",
                    "info",
                ]
                .join("\\n");
                edge_engine
                    .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
                    .await
                    .unwrap();
                edge_engine.commit().await.unwrap();
                let mut global = global.lock().await;
                let rs = global.get_target_v_unchecked("root", "web_server");
                assert!(!rs.is_empty());
            })
    }
}
