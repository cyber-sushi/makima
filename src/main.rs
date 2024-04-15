mod config;
mod virtual_devices;
mod event_reader;
mod udev_monitor;
mod active_client;

use std::env;
use tokio;
use home;
use config::Config;
use tokio::task::JoinHandle;
use crate::udev_monitor::*;


#[tokio::main]
async fn main() {
    let default_config_path = format!("{}/.config/makima", home::home_dir().unwrap().display());
    let env_path: String = match env::var("MAKIMA_CONFIG") {
        Ok(path) => {
            println!("\nConfig directory set to {:?}.", path);
            path
        },
        Err(_) => {
            println!("\n\"MAKIMA_CONFIG\" environment variable is not set, defaulting to {:?}.", default_config_path);
            default_config_path.clone()
        },
    };
    let config_path: std::fs::ReadDir = match std::fs::read_dir(&env_path) {
        Ok(config_path) => {
            println!("Scanning for config files...\n");
            config_path
        },
        Err(_) => {
            println!("Directory not found, falling back to {:?}.\n", default_config_path);
            std::fs::read_dir(default_config_path).unwrap()
        },
    };
    let mut config_files: Vec<Config> = Vec::new();
    for file in config_path {
        let filename: String = file.as_ref().unwrap().file_name().into_string().unwrap();
        if filename.ends_with(".toml") && !filename.starts_with(".") {
            let name: String = filename.split(".toml").collect::<Vec<&str>>()[0].to_string();
            let config_file: Config = Config::new_from_file(file.unwrap().path().to_str().unwrap(), name);
            config_files.push(config_file);
        }
    }
    let tasks: Vec<JoinHandle<()>> = Vec::new();
    start_monitoring_udev(config_files, tasks).await;
}
