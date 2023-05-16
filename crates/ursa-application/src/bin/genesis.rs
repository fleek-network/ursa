use std::{env, fs, time::SystemTime};

use ursa_application::genesis::Genesis;

const GENESIS_PATH: &str = "crates/ursa-application/genesis.toml";
fn main() {
    let args: Vec<String> = env::args().collect();
    let epoch_time = match args.get(1) {
        Some(time) => time,
        None => "300000",
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mut genesis = Genesis::load().unwrap();

    genesis.epoch_start = now as u64;
    genesis.epoch_time = epoch_time.parse().unwrap();

    let genesis_toml = toml::to_string(&genesis).unwrap();
    fs::write(env::current_dir().unwrap().join(GENESIS_PATH), genesis_toml).unwrap();
}
