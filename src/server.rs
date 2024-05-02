//! Server that provides services.
mod middle_ware;
mod service;

use std::io;

use actix_web::{web, HttpServer};
use edge_lib::{data::DataManager, AsEdgeEngine, EdgeEngine};

// Public
pub struct WebServer {
    dm: DataManager,
    // ip: String,
    // name: String,
    // port: u16,
    // path: String,
    // src: String,
    // proxy: Arc<Mutex<BTreeMap<String, String>>>,
    // moon_server_v: Vec<String>,
}

impl WebServer {
    pub fn new(dm: DataManager) -> Self {
        Self { dm }
    }

    /// Server run itself. This will block current thread.
    pub async fn run(self) -> io::Result<()> {
        let mut edge_engine = EdgeEngine::new(self.dm.clone());

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
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .app_data(web::Data::new(self.dm.clone()))
                .wrap(middle_ware::Proxy::new())
                .service(service::config(&path, &src))
        });
        server.bind(&domain)?.run().await
    }
}
