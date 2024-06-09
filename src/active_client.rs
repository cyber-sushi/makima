use crate::udev_monitor::{Client, Environment, Server};
use crate::Config;
use serde_json;
use std::process::{Command, Stdio};
use swayipc_async::Connection;
use x11rb::protocol::xproto::{get_input_focus, get_property, Atom, AtomEnum};

pub async fn get_active_window(environment: &Environment, config: &Vec<Config>) -> Client {
    match &environment.server {
        Server::Connected(server) => {
            let server_str = server.as_str();
            match server_str {
                "Hyprland" => {
                    let query = Command::new("hyprctl")
                        .args(["activewindow", "-j"])
                        .output()
                        .unwrap();
                    if let Ok(reply) = serde_json::from_str::<serde_json::Value>(
                        std::str::from_utf8(query.stdout.as_slice()).unwrap(),
                    ) {
                        let active_window =
                            Client::Class(reply["class"].to_string().replace("\"", ""));
                        if let Some(_) = config
                            .iter()
                            .find(|&x| x.associations.client == active_window)
                        {
                            active_window
                        } else {
                            Client::Default
                        }
                    } else {
                        Client::Default
                    }
                }
                "sway" => {
                    let mut connection = Connection::new().await.unwrap();
                    let active_window = match connection
                        .get_tree()
                        .await
                        .unwrap()
                        .find_focused(|window| window.focused)
                    {
                        Some(window) => match window.app_id {
                            Some(id) => Client::Class(id),
                            None => Client::Class(window.window_properties.unwrap().class.unwrap()),
                        },
                        None => Client::Default,
                    };
                    if let Some(_) = config
                        .iter()
                        .find(|&x| x.associations.client == active_window)
                    {
                        active_window
                    } else {
                        Client::Default
                    }
                }
                "KDE" => {
                    let (user, running_as_root) =
                        if let Ok(sudo_user) = environment.sudo_user.clone() {
                            (Option::Some(sudo_user), true)
                        } else if let Ok(user) = environment.user.clone() {
                            (Option::Some(user), false)
                        } else {
                            (Option::None, false)
                        };
                    let active_window = {
                        if let Some(user) = user {
                            if running_as_root {
                                let output = Command::new("runuser")
                                    .arg(user)
                                    .arg("-c")
                                    .arg("kdotool getactivewindow getwindowclassname")
                                    .output()
                                    .unwrap();
                                Client::Class(
                                    std::str::from_utf8(output.stdout.as_slice())
                                        .unwrap()
                                        .trim()
                                        .to_string(),
                                )
                            } else {
                                let output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("systemd-run --user --scope -M {}@ kdotool getactivewindow getwindowclassname", user))
                                    .stderr(Stdio::null())
                                    .output()
                                    .unwrap();
                                Client::Class(
                                    std::str::from_utf8(output.stdout.as_slice())
                                        .unwrap()
                                        .trim()
                                        .to_string(),
                                )
                            }
                        } else {
                            Client::Default
                        }
                    };
                    if let Some(_) = config
                        .iter()
                        .find(|&x| x.associations.client == active_window)
                    {
                        active_window
                    } else {
                        Client::Default
                    }
                }
                "x11" => {
                    let connection = x11rb::connect(None).unwrap().0;
                    let focused_window =
                        get_input_focus(&connection).unwrap().reply().unwrap().focus;
                    let (wm_class, string): (Atom, Atom) =
                        (AtomEnum::WM_CLASS.into(), AtomEnum::STRING.into());
                    let class = get_property(
                        &connection,
                        false,
                        focused_window,
                        wm_class,
                        string,
                        0,
                        u32::MAX,
                    )
                    .unwrap()
                    .reply()
                    .unwrap()
                    .value;
                    if let Some(middle) = class.iter().position(|&byte| byte == 0) {
                        let class = class.split_at(middle).1;
                        let mut class = &class[1..];
                        if class.last() == Some(&0) {
                            class = &class[..class.len() - 1];
                        }
                        let active_window =
                            Client::Class(std::str::from_utf8(class).unwrap().to_string());
                        if let Some(_) = config
                            .iter()
                            .find(|&x| x.associations.client == active_window)
                        {
                            active_window
                        } else {
                            Client::Default
                        }
                    } else {
                        Client::Default
                    }
                }
                _ => Client::Default,
            }
        }
        Server::Unsupported => Client::Default,
        Server::Failed => Client::Default,
    }
}
