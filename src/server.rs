//! Server that provides services.
mod middle_ware;
mod service;

use std::{io, sync::Arc};

use actix_web::{web, HttpServer};
use edge_lib::{data::AsDataManager, AsEdgeEngine, EdgeEngine};

// Public
pub struct WebServer {
    dm: Box<dyn AsDataManager>,
}

impl WebServer {
    pub fn new(dm: Box<dyn AsDataManager>) -> Self {
        Self { dm }
    }

    /// Server run itself. This will block current thread.
    pub async fn run(self) -> io::Result<()> {
        let mut edge_engine = EdgeEngine::new(self.dm.divide());

        let script = [
            "$->$output = = root->name _",
            "$->$output += = root->ip _",
            "$->$output += = root->port _",
            "$->$output += = root->path _",
            "$->$output += = root->src _",
            "info",
        ]
        .join("\\n");
        let rs = edge_engine
            .execute(&json::parse(&format!("{{\"{script}\": null}}")).unwrap())
            .await?;
        log::debug!("{rs}");
        let name = rs["info"][0].as_str().unwrap();
        let ip = rs["info"][1].as_str().unwrap();
        let port = rs["info"][2].as_str().unwrap();
        let path = rs["info"][3].as_str().unwrap().to_string();
        let src = rs["info"][4].as_str().unwrap().to_string();

        let domain = format!("{ip}:{port}");
        log::info!("http service {name} uri: http://{domain}{path}");
        let dm = Arc::new(self.dm);
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .app_data(web::Data::new(dm.divide()))
                .wrap(middle_ware::Proxy::new())
                .service(service::config(&path, &src))
        });
        server.bind(&domain)?.run().await
    }
}