use std::sync::OnceLock;

use tracing::{level_filters::LevelFilter, warn};

pub const TEST_SERVICE: &str = "tiny_daemon.service";

//Too avoid duplicate init during series os tests
static TRACING: OnceLock<()> = OnceLock::new();

pub fn init_logs() {
    TRACING.get_or_init(|| {
        let timer =
            tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());

        if let Err(err) = tracing_subscriber::fmt()
            .with_timer(timer)
            .with_max_level(LevelFilter::DEBUG)
            .try_init()
        {
            warn!("Tracing Init Error:{:?}", err)
        }
    });
}
