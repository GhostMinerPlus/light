mod http;

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

pub async fn init(domain: &str, path: &str, hosts: &Vec<String>) -> io::Result<()> {
    // init
    http::init(domain, path, hosts).await
}

pub async fn run(
    domain: &str,
    path: &str,
    src: &str,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
) -> io::Result<()> {
    http::run(domain, path.to_string(), src.to_string(), proxy).await
}
