pub struct StdLooger {}

impl StdLooger {}

pub const STD_LOGGER: StdLooger = StdLooger {};

impl log::Log for StdLooger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{:?}-{}: {}",
                time::OffsetDateTime::now_utc(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
