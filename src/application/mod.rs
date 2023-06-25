pub(crate) mod dto;

use std::cmp::min;

use crate::App;

pub(crate) async fn add_proxy(proxy: &dto::Proxy) -> String {
    let mut proxies = App::get_app().proxies.lock().unwrap();
    proxies.insert(proxy.path.clone(), proxy.url.clone());
    proxy.path.clone()
}

pub(crate) async fn remove_proxy(name: &str) -> String {
    let mut proxies = App::get_app().proxies.lock().unwrap();
    proxies.remove(name);
    name.to_string()
}

pub(crate) async fn list_proxies(page: &dto::Page) -> Vec<dto::Proxy> {
    let proxies = App::get_app().proxies.lock().unwrap();

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
