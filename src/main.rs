mod config;
mod virtual_devices;
mod event_reader;
mod udev_monitor;

use tokio;
use home;
use config::Config;
use tokio::task::JoinHandle;
use crate::udev_monitor::*;


#[tokio::main]
async fn main() {
    let config_path = std::fs::read_dir(format!("{}/.config/makima", home::home_dir().unwrap().display())).unwrap();
    let mut config_files: Vec<Config> = Vec::new();
    for file in config_path {
        let filename: String = file.as_ref().unwrap().file_name().into_string().unwrap()
            .split(".toml").collect::<Vec<&str>>()[0].to_string();
        let config_file: Config = Config::new_from_file(file.unwrap().path().to_str().unwrap(), filename);
        config_files.push(config_file);
    }
    let tasks: Vec<JoinHandle<()>> = Vec::new();
    start_monitoring_udev(config_files, tasks).await;
}
