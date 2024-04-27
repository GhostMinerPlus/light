use std::sync::Arc;

use actix_files::{Files, NamedFile};
use actix_web::{dev::{HttpServiceFactory, ServiceRequest, ServiceResponse}, web, HttpResponse, Responder};
use edge_lib::{data::DataManager, mem_table::MemTable, AsEdgeEngine, EdgeEngine};
use tokio::sync::Mutex;

#[actix_web::post("/execute")]
async fn execute(global: web::Data<Arc<Mutex<MemTable>>>, script: String) -> impl Responder {
    let mut edge_engine = EdgeEngine::new(DataManager::with_global((**global).clone()));
    let rs = edge_engine.execute(&json::parse(&script).unwrap()).await.unwrap();
    // let mut proxies = ctx.proxy.lock().unwrap();
    // proxies.insert(proxy.path.clone(), proxy.url.clone());
    HttpResponse::Ok()
        .content_type("application/json")
        .body(rs.to_string())
}

pub fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path)
        .service(execute)
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
