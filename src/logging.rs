use env_logger::Builder;
use log::LevelFilter;
use std::env;

pub fn init() {
    let mut builder = Builder::from_default_env();
    builder
        .format_timestamp_secs()
        .format_module_path(false)
        .format_target(false)
        .filter_level(LevelFilter::Info);

    if env::var("RUST_LOG").is_err() {
        builder.filter_module("sentinel", LevelFilter::Info);
    }

    let _ = builder.try_init();
}
