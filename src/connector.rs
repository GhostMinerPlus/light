use std::{io, sync::Arc, time::Duration};

use edge_lib::{data::AsDataManager, util::Path, EdgeEngine, ScriptTree};
use tokio::time;

use crate::util;

pub struct HttpConnector {
    dm: Arc<dyn AsDataManager>,
}

impl HttpConnector {
    pub fn new(dm: Arc<dyn AsDataManager>) -> Self {
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
        let mut edge_engine = EdgeEngine::new(self.dm.clone());

        let rs = edge_engine
            .execute1(&ScriptTree {
                script: [
                    "$->$output = root->name _",
                    "$->$output append $->$output root->port",
                    "$->$output append $->$output root->path",
                ]
                .join("\n"),
                name: format!("info"),
                next_v: vec![],
            })
            .await
            .map_err(|e| io::Error::other(format!("{e}\nwhen execute")))?;
        log::debug!("{rs}");
        let name = rs["info"][0].as_str().unwrap();
        let ip = {
            let domain_v = self.dm.get(&Path::from_str("root->domain")).await?;
            if !domain_v.is_empty() && !domain_v[0].is_empty() {
                domain_v[0].clone()
            } else {
                util::native::get_global_ipv6()?
            }
        };
        let port = rs["info"][1].as_str().unwrap();
        let path = rs["info"][2].as_str().unwrap();

        let rs = edge_engine
            .execute1(&ScriptTree {
                script: ["$->$output = root->moon_server _"].join("\n"),
                name: format!("moon_server"),
                next_v: vec![],
            })
            .await
            .map_err(|e| io::Error::other(format!("{e}\nwhen execute")))?;
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
                log::warn!("when execute:\nwhen http_execute:\n{e}");
                if let Err(e) = util::http_execute1(
                    &uri,
                    &ScriptTree {
                        script: script.replace("\\n", "\n"),
                        name: format!("info"),
                        next_v: vec![],
                    },
                )
                .await
                {
                    log::warn!("when execute:\nwhen http_execute1:\n{e}");
                }
            } else {
                log::info!("reported to {uri}");
            }
        }
        Ok(())
    }
}
