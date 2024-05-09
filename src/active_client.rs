use crate::Config;
use serde_json;
use swayipc_async::Connection;
use std::{collections::HashMap, process::Command};
use x11rb::protocol::xproto::{get_property, get_input_focus, Atom, AtomEnum};

pub async fn get_active_window(current_desktop: &Option<String>, config: &HashMap<String, Config>) -> String {
    let active_client = current_desktop.clone().unwrap_or(String::from("default"));
    match active_client.as_str() {
        "Hyprland" => {
            let query = Command::new("hyprctl").args(["activewindow", "-j"]).output().unwrap();
            if let Ok(reply) = serde_json::from_str::<serde_json::Value>(std::str::from_utf8(query.stdout.as_slice()).unwrap()) {
                let active_window = reply["class"].to_string().replace("\"", "");
                if config.contains_key(&active_window) {
                    active_window
                } else {
                    String::from("default")
                }
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
            let connection = x11rb::connect(None).unwrap().0;
            let focused_window = get_input_focus(&connection)
                .unwrap().reply().unwrap().focus;
            let (wm_class, string): (Atom, Atom) = (AtomEnum::WM_CLASS.into(), AtomEnum::STRING.into());
            let class = get_property(&connection, false, focused_window, wm_class, string, 0, u32::MAX)
                .unwrap().reply().unwrap().value;
            if let Some(middle) = class.iter().position(|&byte| byte == 0) {
                let class = class.split_at(middle).1;
                let mut class = &class[1..];
                if class.last() == Some(&0) {
                    class = &class[..class.len() -1];
                }
                let active_window = std::str::from_utf8(class).unwrap().to_string();
                if config.contains_key(&active_window) {
                    active_window
                } else {
                    String::from("default")
                }
            } else {
                String::from("default")
            }
        },
        _ => String::from("default")
    }
}

