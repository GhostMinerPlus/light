use actix_files::{Files, NamedFile};
use actix_web::dev::{HttpServiceFactory, ServiceRequest, ServiceResponse};

pub mod dto;
pub mod system;

pub fn config(path: &str, src: &str) -> impl HttpServiceFactory {
    let src = src.to_string();
    actix_web::web::scope(&path)
        .service(system::add_proxy)
        .service(system::remove_proxy)
        .service(system::list_proxies)
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
