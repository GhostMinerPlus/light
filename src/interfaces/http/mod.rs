use actix_files::NamedFile;
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    web,
};

use crate::App;

pub(crate) fn config(cfg: &mut web::ServiceConfig) {
    let src = App::get_config().src.clone();

    cfg.service(
        actix_files::Files::new("", &src)
            .index_file("index.html")
            .default_handler(fn_service(|req: ServiceRequest| async {
                let (req, _) = req.into_parts();
                let file =
                    NamedFile::open_async(&format!("{}/index.html", App::get_config().src)).await?;
                let res = file.into_response(&req);
                Ok(ServiceResponse::new(req, res))
            })),
    );
}
