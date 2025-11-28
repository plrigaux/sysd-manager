use tracing_subscriber::filter::LevelFilter;

pub fn init_logs() {
    /*     let _ = env_logger::builder()
    .target(env_logger::Target::Stdout)
    .filter_level(log::LevelFilter::Debug)
    .is_test(true)
    .try_init(); */

    let timer = tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());
    //let timer = fmt::time::ChronoLocal::rfc_3339();

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_max_level(LevelFilter::DEBUG)
        .init();
}
