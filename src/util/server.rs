//! Server that provides services.
mod middle_ware;
mod service;

use std::{io, sync::Arc};

use actix_web::{web, HttpServer};
use edge_lib::util::{
    data::MemDataManager,
    engine::{AsEdgeEngine, EdgeEngine},
};
use tokio::sync::Mutex;

// Public
pub struct WebServer {
    global: Arc<Mutex<MemDataManager>>,
}

impl WebServer {
    pub fn new(global: Arc<Mutex<MemDataManager>>) -> Self {
        Self { global }
    }

    /// Server run itself. This will block current thread.
    pub async fn run(self) -> io::Result<()> {
        let mut global = self.global.lock().await;

        let mut edge_engine = EdgeEngine::new(&mut *global);

        let rs = edge_engine
            .execute_script(&[
                "$->$:output = root->name _".to_string(),
                "$->$:output += $->$:output root->ip".to_string(),
                "$->$:output += $->$:output root->port".to_string(),
                "$->$:output += $->$:output root->path".to_string(),
                "$->$:output += $->$:output root->src".to_string(),
            ])
            .await?;

        drop(edge_engine);
        drop(global);

        let name = &rs[0];
        let ip = &rs[1];
        let port = &rs[2];
        let path = rs[3].clone();
        let src = rs[4].clone();

        let domain = format!("{ip}:{port}");
        log::info!("http service {name} uri: http://{domain}{path}");
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .app_data(web::Data::new(self.global.clone()))
                .wrap(middle_ware::Proxy::new())
                .service(service::config(&path, &src))
        });
        server.bind(&domain)?.run().await
    }
}
