mod http;

use std::io;

use crate::util::Context;

pub async fn init(domain: &str, path: &str, hosts: &Vec<String>) -> io::Result<()> {
    // init
    http::init(domain, path, hosts).await
}

pub async fn run(ctx: Context) -> io::Result<()> {
    http::run(ctx).await
}
