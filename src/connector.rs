use std::{
    io,
    time::Duration,
};

use edge_lib::{data::AsDataManager, AsEdgeEngine, EdgeEngine};
use tokio::time;

use crate::util;

pub struct HttpConnector {
    dm: Box<dyn AsDataManager>,
}

impl HttpConnector {
    pub fn new(dm: Box<dyn AsDataManager>) -> Self {
        Self { dm }
    }

    pub async fn run(self) -> io::Result<()> {
        loop {
            if let Err(e) = self.execute().await {
                log::warn!("when run:\n{e}");
            }

            time::sleep(Duration::from_secs(10)).await;
        }
    }

    async fn execute(&self) -> io::Result<()> {
        let mut edge_engine = EdgeEngine::new(self.dm.divide());

        let script = [
            "$->$output = = root->name _",
            "$->$output += = root->port _",
            "$->$output += = root->path _",
            "info",
        ]
        .join("\\n");
        let rs = edge_engine
            .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
            .await
            .map_err(|e| io::Error::other(format!("when execute:\n{e}")))?;
        log::debug!("{rs}");
        let name = rs["info"][0].as_str().unwrap();
        let ip = util::native::get_global_ipv6()?;
        let port = rs["info"][1].as_str().unwrap();
        let path = rs["info"][2].as_str().unwrap();

        let script = ["$->$output = = root->moon_server _", "moon_server"].join("\\n");
        let rs = edge_engine
            .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
            .await
            .map_err(|e| io::Error::other(format!("when execute:\n{e}")))?;
        log::debug!("{rs}");
        let moon_server_v = &rs["moon_server"];

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
        for moon_server in moon_server_v.members() {
            let uri = match moon_server.as_str() {
                Some(uri) => uri,
                None => {
                    log::error!("when execute:\nfailed to parse uri for moon_server");
                    continue;
                }
            };
            log::info!("reporting to {uri}");
            if let Err(e) = util::http_execute(&uri, format!("{{\"{script}\": null}}")).await {
                log::warn!("when execute:\n{e}");
            } else {
                log::info!("reported to {uri}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use edge_lib::{data::{AsDataManager, DataManager}, AsEdgeEngine, EdgeEngine};

    #[test]
    fn test() {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut dm = DataManager::new();
                let mut edge_engine = EdgeEngine::new(dm.divide());
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
                let rs = dm.get_target_v("root", "web_server").await.unwrap();
                assert!(!rs.is_empty());
            })
    }
}