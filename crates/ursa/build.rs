use std::path::PathBuf;

mod config {
    include!("src/config.rs");
}
use config::{load_config, DEFAULT_CONFIG_PATH_STR};


fn main() {
    let _ = load_config(&PathBuf::from(env!("HOME")).join(DEFAULT_CONFIG_PATH_STR));
}
