pub mod api;
pub mod app;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            api::init().await;
            app::run().await;
        });
}
