use std::sync::Arc;

use actix_files::{Files, NamedFile};
use actix_web::{
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    web, HttpResponse, Responder,
};
use edge_lib::{data::AsDataManager, engine::EdgeEngine};

#[actix_web::post("/execute")]
async fn execute(dm: web::Data<Arc<dyn AsDataManager>>, script: String) -> impl Responder {
    let mut edge_engine = EdgeEngine::new(dm.as_ref().clone(), "root").await;
    let rs = edge_engine
        .execute(&json::parse(&script).unwrap())
        .await
        .unwrap();
    edge_engine.commit().await.unwrap();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(rs.to_string())
}

#[actix_web::post("/execute1")]
async fn execute1(dm: web::Data<Arc<dyn AsDataManager>>, script: String) -> impl Responder {
    let mut edge_engine = EdgeEngine::new(dm.as_ref().clone(), "root").await;
    let rs = edge_engine
        .execute1(&serde_json::from_str(&script).unwrap())
        .await
        .unwrap();
    edge_engine.commit().await.unwrap();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(rs.to_string())
}

pub fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path)
        .service(execute)
        .service(execute1)
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
