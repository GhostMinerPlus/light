pub mod service;

use actix_files::{Files, NamedFile};
use actix_web::{
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    web, HttpServer,
};
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};

mod middle_ware;

struct Context {
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
}

fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path)
        .service(service::system::add_proxy)
        .service(service::system::remove_proxy)
        .service(service::system::list_proxies)
        .service(
            Files::new("", &src)
                .index_file("index.html")
                .default_handler(actix_web::dev::fn_service(move |req: ServiceRequest| {
                    let index_html = format!("{}/index.html", src);
                    let (req, _) = req.into_parts();
                    async {
                        let file = NamedFile::open_async(index_html).await?;
                        let res = file.into_response(&req);
                        Ok(ServiceResponse::new(req, res))
                    }
                })),
        )
}

// public
pub async fn run(
    domain: &str,
    path: String,
    src: String,
    proxy: Arc<Mutex<BTreeMap<String, String>>>,
) -> io::Result<()> {
    log::info!("http service uri: http://{domain}{path}");

    let server = HttpServer::new(move || {
        actix_web::App::new()
            .app_data(web::Data::new(Context {
                proxy: proxy.clone(),
            }))
            .wrap(middle_ware::Proxy::new())
            .service(config(&path, &src))
    });
    server.bind(&domain)?.run().await
}
