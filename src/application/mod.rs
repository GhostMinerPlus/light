pub mod dto;

use std::cmp::min;

use crate::infrastructure::Context;

pub async fn add_proxy(proxy: &dto::Proxy) -> String {
    let mut proxies = Context::as_ref().proxies.lock().unwrap();
    proxies.insert(proxy.path.clone(), proxy.url.clone());
    proxy.path.clone()
}

pub async fn remove_proxy(name: &str) -> String {
    let mut proxies = Context::as_ref().proxies.lock().unwrap();
    proxies.remove(name);
    name.to_string()
}

pub async fn list_proxies(page: &dto::Page) -> Vec<dto::Proxy> {
    let proxies = Context::as_ref().proxies.lock().unwrap();

    let start = page.number * page.size;
    let end = min(start + page.size, proxies.len());

    let mut list = Vec::new();
    let mut i = 0;
    for proxy in &*proxies {
        if i >= start {
            list.push(dto::Proxy {
                path: proxy.0.clone(),
                url: proxy.1.clone(),
            });
        }

        i += 1;

        if i >= end {
            break;
        }
    }
    list
}
