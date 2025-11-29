use tracing::level_filters::LevelFilter;

pub const TEST_SERVICE: &str = "tiny_daemon.service";

pub fn init_logs() {
    let timer = tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_max_level(LevelFilter::DEBUG)
        .init();
}
