//! Server that provides services.

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::time;

use crate::star;

mod service;

async fn serve(
    name: &str,
    domain: &str,
    path: &str,
    src: &str,
    hosts: &Vec<String>,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
) -> io::Result<()> {
    service::init(domain, path, hosts).await?;
    log::info!("{} starting", name);
    service::run(domain, path, src, proxy).await
}

// Public
pub struct Server {
    ip: String,
    name: String,
    port: u16,
    path: String,
    src: String,
    hosts: Vec<String>,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
    moon_server_v: Vec<String>,
}

impl Server {
    pub fn new(
        ip: String,
        port: u16,
        path: String,
        name: String,
        src: String,
        hosts: Vec<String>,
        proxy: BTreeMap<String, String>,
        moon_server_v: Vec<String>,
    ) -> Self {
        Self {
            ip,
            port,
            path,
            name,
            src,
            hosts,
            proxy: Arc::new(Mutex::new(proxy)),
            moon_server_v,
        }
    }

    pub async fn run(self) -> io::Result<()> {
        let name = self.name.clone();
        let path = self.path.clone();
        tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_secs(10)).await;
                if let Err(e) = star::report_uri(&name, self.port, &path, &self.moon_server_v).await
                {
                    log::error!("{e}");
                }
            }
        });
        serve(
            &self.name,
            &format!("{}:{}", self.ip, self.port),
            &self.path,
            &self.src,
            &self.hosts,
            self.proxy.clone(),
        )
        .await
    }
}
