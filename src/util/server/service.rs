use std::sync::Arc;

use actix_files::{Files, NamedFile};
use actix_web::{
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    web, HttpResponse, Responder,
};
use edge_lib::util::{
    data::MemDataManager,
    engine::{AsEdgeEngine, EdgeEngine},
};
use tokio::sync::Mutex;

#[actix_web::post("/execute")]
async fn execute(
    global_mutex: web::Data<Arc<Mutex<MemDataManager>>>,
    script: String,
) -> impl Responder {
    let mut global = global_mutex.lock().await;
    let mut edge_engine = EdgeEngine::new(&mut *global);
    let rs = edge_engine
        .execute_script(&serde_json::from_str::<'_, Vec<String>>(&script).unwrap())
        .await
        .unwrap();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&rs).unwrap())
}

pub fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path).service(execute).service(
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
