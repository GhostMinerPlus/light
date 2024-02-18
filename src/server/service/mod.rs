mod http;

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

// Public
pub async fn run(
    domain: &str,
    path: &str,
    src: &str,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
    moon_server_v: Vec<String>,
) -> io::Result<()> {
    http::run(
        domain,
        path.to_string(),
        src.to_string(),
        proxy,
        moon_server_v,
    )
    .await
}
