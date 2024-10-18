use std::{io, sync::Arc, time::Duration};

use edge_lib::util::{
    data::{AsDataManager, MemDataManager},
    engine::{AsEdgeEngine, EdgeEngine},
    Path,
};
use tokio::{sync::Mutex, time};

use crate::util;

const SLEEP_TIME: Duration = Duration::from_secs(10);

pub struct HttpConnector {
    global: Arc<Mutex<MemDataManager>>,
}

impl HttpConnector {
    pub fn new(global: Arc<Mutex<MemDataManager>>) -> Self {
        Self { global }
    }

    pub async fn run(self) -> io::Result<()> {
        loop {
            if let Err(e) = self.execute().await {
                log::warn!("{e}\nwhen run");
            }

            time::sleep(SLEEP_TIME).await;
        }
    }

    async fn execute(&self) -> io::Result<()> {
        let mut global = self.global.lock().await;
        let domain_v = global
            .get(&Path::from_str("root->domain"))
            .await
            .map_err(|e| io::Error::other(format!("{e:?}\nwhen execute")))?;
        let ip = if domain_v.is_empty() {
            util::native::get_global_ipv6()
                .map_err(|e| io::Error::other(format!("{e}\nwhen execute")))?
        } else {
            domain_v[0].clone()
        };

        let mut edge_engine = EdgeEngine::new(&mut *global);
        let rs = edge_engine
            .execute_script(&[
                format!("$->$:output = root->name _"),
                format!("$->$:output append $->$:output root->port"),
                format!("$->$:output append $->$:output root->path"),
            ])
            .await
            .map_err(|e| io::Error::other(format!("{e:?}\nwhen execute")))?;

        let name = &rs[0];
        let port = &rs[1];
        let path = &rs[2];

        let moon_server_v = edge_engine
            .execute_script(&[format!("$->$:output = root->moon_server _")])
            .await
            .map_err(|e| io::Error::other(format!("{e:?}\nwhen execute")))?;
        drop(edge_engine);
        drop(global);

        let script = vec![
            format!("$->$:server_exists inner root->web_server {name}<-name"),
            format!("$->$:web_server if $->$:server_exists ?"),
            format!("$->$:web_server->name = {name} _"),
            format!("$->$:web_server->ip = {ip} _"),
            format!("$->$:web_server->port = {port} _"),
            format!("$->$:web_server->path = {path} _"),
            format!("$->$:web_server left $->$:web_server $->$:server_exists"),
            format!("root->web_server append root->web_server $->$:web_server"),
        ];
        for uri in &moon_server_v {
            log::info!("reporting to {uri}");
            if let Err(e) = util::native::http_execute_script(&uri, &script).await {
                log::warn!("{e}\nwhen execute");
            } else {
                log::info!("reported to {uri}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use edge_lib::util::{
        data::{AsDataManager, MemDataManager},
        engine::{AsEdgeEngine, EdgeEngine},
        Path,
    };

    #[test]
    fn test() {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut global = MemDataManager::new(None);

                let mut edge_engine = EdgeEngine::new(&mut global);
                // config.ip, config.port, config.name
                let name = "test";
                let ip = "0.0.0.0";
                let port = "8080";
                let path = "/test";

                edge_engine
                    .execute_script(&[
                        format!("$->$:server_exists inner root->web_server {name}<-name"),
                        format!("$->$:web_server if $->$:server_exists ?"),
                        format!("$->$:web_server->name = {name} _"),
                        format!("$->$:web_server->ip = {ip} _"),
                        format!("$->$:web_server->port = {port} _"),
                        format!("$->$:web_server->path = {path} _"),
                        format!("$->$:web_server left $->$:web_server $->$:server_exists"),
                        format!("root->web_server append root->web_server $->$:web_server"),
                    ])
                    .await
                    .unwrap();
                drop(edge_engine);

                let rs = global
                    .get(&Path::from_str("root->web_server"))
                    .await
                    .unwrap();
                assert!(!rs.is_empty());
            })
    }
}
