mod active_client;
mod config;
mod event_reader;
mod udev_monitor;
mod virtual_devices;

use crate::udev_monitor::*;
use config::Config;
use std::env;
use tokio;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    let config_path = match env::var("MAKIMA_CONFIG") {
        Ok(path) => {
            println!("\nMAKIMA_CONFIG set to {:?}.\n", path);
            match std::fs::read_dir(path) {
                Ok(dir) => dir,
                _ => {
                    println!("Directory not found, exiting Makima.");
                    std::process::exit(0);
                }
            }
        }
        Err(_) => {
            let user_home = match env::var("HOME") {
                Ok(user_home) if user_home == "/root".to_string() => match env::var("SUDO_USER") {
                    Ok(sudo_user) => format!("/home/{}", sudo_user),
                    _ => user_home,
                },
                Ok(user_home) => user_home,
                _ => "/root".to_string(),
            };
            let default_config_path = format!("{}/.config/makima", user_home);
            println!(
                "\nMAKIMA_CONFIG environment variable is not set, defaulting to {:?}.\n",
                default_config_path
            );
            match std::fs::read_dir(default_config_path) {
                Ok(dir) => dir,
                _ => {
                    println!("Directory not found, exiting Makima.");
                    std::process::exit(0);
                }
            }
        }
    };
    let mut config_files: Vec<Config> = Vec::new();
    for file in config_path {
        let filename: String = file.as_ref().unwrap().file_name().into_string().unwrap();
        if filename.ends_with(".toml") && !filename.starts_with(".") {
            let name: String = filename.split(".toml").collect::<Vec<&str>>()[0].to_string();
            let config_file: Config =
                Config::new_from_file(file.unwrap().path().to_str().unwrap(), name);
            config_files.push(config_file);
        }
    }
    let tasks: Vec<JoinHandle<()>> = Vec::new();
    start_monitoring_udev(config_files, tasks).await;
}
