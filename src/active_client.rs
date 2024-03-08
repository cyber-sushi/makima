use crate::Config;
use hyprland::{data::Client, prelude::*};
use swayipc_async::Connection;
use std::collections::HashMap;

pub async fn get_active_window(current_desktop: &Option<String>, config: &HashMap<String, Config>) -> String {
    let active_client = current_desktop.clone().unwrap_or(String::from("default"));
    match active_client.as_str() {
        "Hyprland" => {
            let active_window: String = match Client::get_active_async().await.unwrap() {
                Some(window) => window.class,
                None => String::from("default")
            };
            if config.contains_key(&active_window) {
                active_window
            } else {
                String::from("default")
            }
        },
        "sway" => {
            let mut connection = Connection::new().await.unwrap();
            let active_window = match connection.get_tree().await.unwrap().find_focused(|window| window.focused) {
                Some(window) => {
                    match window.app_id {
                        Some(id) => id,
                        None => window.window_properties.unwrap().class.unwrap()
                    }
                },
                None => String::from("default")
            };
            if config.contains_key(&active_window) {
                active_window
            } else {
                String::from("default")
            }
        },
        "x11" => {
            String::from("default")
        },
        _ => String::from("default")
    }
}
