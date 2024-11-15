use log4rs::config::{Deserializers, RawConfig};
use log4rs::{init_file, init_raw_config};
use std::process::exit;

pub fn init_logging(log_config: Option<String>) {
    let deserializers = Deserializers::default();
    if let Some(config_path) = log_config {
        init_file(config_path.clone(), deserializers).unwrap_or_else(|error| {
            eprintln!("Failed to parse {config_path}: {error}");
            exit(1);
        });
    } else {
        let config = include_str!("default_logging.yml");
        let config = serde_yaml::from_str::<RawConfig>(config).unwrap();
        init_raw_config(config).unwrap();
    }
}
