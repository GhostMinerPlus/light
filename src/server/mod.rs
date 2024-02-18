//! Server that provides services.

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

mod service;

// Public
pub struct Server {
    name: String,
    domain: String,
    path: String,
    src: String,
    hosts: Vec<String>,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
}

impl Server {
    pub fn new(
        domain: String,
        path: String,
        name: String,
        src: String,
        hosts: Vec<String>,
        proxy: BTreeMap<String, String>,
    ) -> Self {
        Self {
            domain,
            path,
            name,
            src,
            hosts,
            proxy: Arc::new(Mutex::new(proxy)),
        }
    }

    pub async fn run(self) -> io::Result<()> {
        service::init(&self.domain, &self.path, &self.hosts).await?;
        log::info!("{} starting", self.name);
        service::run(&self.domain, &self.path, &self.src, self.proxy.clone()).await
    }
}
