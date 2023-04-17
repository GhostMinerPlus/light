use app::App;
use earth::Config;

mod app;
mod logger;

fn main() {
    let _ =
        log::set_logger(&logger::STD_LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info));
    let mut config = app::Config::default();
    config.merge_by_file("earth.toml");
    config.merge_by_args(&std::env::args().collect());
    App::create_app(config).run();
}
