use std::{collections::HashMap, sync::Arc, option::Option, env};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use evdev::{EventStream, Key, RelativeAxisType, AbsoluteAxisType, EventType, InputEvent};
use hyprland::{data::Client, prelude::*};
use swayipc_async::Connection;
use crate::virtual_devices::VirtualDevices;
use crate::Config;


pub struct EventReader {
    config: HashMap<String, Config>,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    analog_position: Arc<Mutex<Vec<i32>>>,
    device_is_connected: Arc<Mutex<bool>>,
    current_desktop: Option<String>,
}

impl EventReader {
    pub fn new(config: HashMap<String, Config>, stream: Arc<Mutex<EventStream>>, virt_dev: Arc<Mutex<VirtualDevices>>) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {position_vector.push(i)};
        let position_vector_mutex = Arc::new(Mutex::new(position_vector));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        let current_desktop: Option<String> = match env::var("XDG_CURRENT_DESKTOP") {
                Ok(desktop) if vec!["Hyprland".to_string(), "sway".to_string()].contains(&desktop)  => {
                    println!("Running on {}, active window detection enabled.", desktop);
                    Option::Some(desktop)
                },
                Ok(desktop) => {
                    println!("Unsupported desktop: {}, won't be able to change bindings according to active window.\n
                            Currently supported desktops: Hyprland", desktop);
                    Option::None
                },
                Err(_) => {
                    println!("Unable to retrieve current desktop based on XDG_CURRENT_DESKTOP env var.\n
                            Won't be able to change bindings according to active window.");
                    Option::None
                },
        };
        Self {
            config: config,
            stream: stream,
            virt_dev: virt_dev,
            analog_position: position_vector_mutex,
            device_is_connected: device_is_connected,
            current_desktop: current_desktop,
        }
    }

    pub async fn start(&self) {
        let mut stream = self.stream.lock().await;
        let mut analog_mode: &str = "left";
        if let Some(stick) = self.config.get(&self.get_active_window().await).unwrap().settings.get("POINTER_STICK") {
            analog_mode = stick.as_str();
        }
        let mut has_signed_axis_value: &str = "false";
        if let Some(axis_value) = self.config.get(&self.get_active_window().await).unwrap().settings.get("SIGNED_AXIS_VALUE") {
            has_signed_axis_value = axis_value.as_str();
        }
        while let Some(Ok(event)) = stream.next().await {
            match (event.event_type(), RelativeAxisType(event.code()), AbsoluteAxisType(event.code()), analog_mode) {
                (EventType::KEY, _, _, _) => {
                    if let Some(event_list) = self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key(event.code())) {
                        self.emit_event(event_list, event.value()).await
                    } else {
                        self.emit_default_event(event).await;
                    }
                },
                (_, RelativeAxisType::REL_WHEEL | RelativeAxisType::REL_WHEEL_HI_RES, _, _) => {
                    let event_list_option: Option<&Vec<Key>> = match event.value() {
                        -1 => self.config.get(&self.get_active_window().await).unwrap().rel.get(&"SCROLL_WHEEL_DOWN".to_string()),
                        1 => self.config.get(&self.get_active_window().await).unwrap().rel.get(&"SCROLL_WHEEL_UP".to_string()),
                        _ => None,
                    };
                    if let Some(event_list) = event_list_option {
                        self.emit_event(event_list, event.value()).await;
                        self.emit_event(event_list, 0).await;
                    } else {
                        if !self.config.get(&self.get_active_window().await).unwrap().rel.contains_key("SCROLL_WHEEL_DOWN")
                        && !self.config.get(&self.get_active_window().await).unwrap().rel.contains_key("SCROLL_WHEEL_UP") {
                            self.emit_default_event(event).await;
                        }
                    }
                },
                (_, _, AbsoluteAxisType::ABS_HAT0X, _) => {
                    let event_list_option: Option<&Vec<Key>> = match event.value() {
                        -1 => self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_DPAD_LEFT),
                        0 => self.config.get(&self.get_active_window().await).unwrap().abs.get(&"NONE_X".to_string()),
                        1 => self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_DPAD_RIGHT),
                        _ => self.config.get(&self.get_active_window().await).unwrap().abs.get(&"NONE_X".to_string()),
                    };
                    if let Some(event_list) = event_list_option {
                        self.emit_event(event_list, event.value()).await;
                    } else {
                        println!("Button not set in the config file!");
                    }
                },
                (_, _, AbsoluteAxisType::ABS_HAT0Y, _) => {
                    let event_list_option: Option<&Vec<Key>> = match event.value() {
                        -1 => self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_DPAD_UP),
                        0 => self.config.get(&self.get_active_window().await).unwrap().abs.get(&"NONE_Y".to_string()),
                        1 => self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_DPAD_DOWN),
                        _ => self.config.get(&self.get_active_window().await).unwrap().abs.get(&"NONE_Y".to_string()),
                    };
                    if let Some(event_list) = event_list_option {
                        self.emit_event(event_list, event.value()).await;
                    } else {
                        println!("Button not set in the config file!");
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y, "left") => {
                    let rel_value = self.get_rel_value(&has_signed_axis_value, &event).await;
                    let mut analog_position = self.analog_position.lock().await;
                    analog_position[event.code() as usize] = rel_value;
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY, "right") => {
                    let rel_value = self.get_rel_value(&has_signed_axis_value, &event).await;
                    let mut analog_position = self.analog_position.lock().await;
                    analog_position[(event.code() as usize) -3] = rel_value;
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_Z, _) => {
                    if let Some(event_list) = self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_TL2) {
                        if event.value() == 0 {
                            self.emit_event(event_list, event.value()).await
                        } else {
                            self.emit_event(event_list, 1).await
                        };
                    } else {
                        println!("Button not set in the config file!");
                    };
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RZ, _) => {
                    if let Some(event_list) = self.config.get(&self.get_active_window().await).unwrap().keys.get(&Key::BTN_TR2) {
                        if event.value() == 0 {
                            self.emit_event(event_list, event.value()).await
                        } else {
                            self.emit_event(event_list, 1).await
                        };
                    } else {
                        println!("Button not set in the config file!");
                    };
                },
                _ => {self.emit_default_event(event).await}
            }
        }
        let mut device_is_connected = self.device_is_connected.lock().await;
        *device_is_connected = false;
    }

    async fn emit_event(&self, event_list: &Vec<Key>, value: i32) {
        for key in event_list {
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), value);
            let mut virt_dev = self.virt_dev.lock().await;
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
    }
    
    async fn emit_default_event(&self, event: InputEvent) {
        let mut virt_dev = self.virt_dev.lock().await;
        match event.event_type() {
            EventType::KEY => virt_dev.keys.emit(&[event]).unwrap(),
            EventType::RELATIVE => virt_dev.relative_axes.emit(&[event]).unwrap(),
            _ => {}
        }
    }
    
    async fn get_rel_value(&self, has_signed_axis_value: &str, event: &InputEvent) -> i32 {
        let rel_value: i32 = match &has_signed_axis_value {
            &"false" => {
                let distance_from_center: i32 = event.value() as i32 - 128;
                distance_from_center / 10
            }
            _ => {
                event.value() as i32 / 2000
            }
        };
        return rel_value
    }

    pub async fn cursor_loop(&self) {
        if let Some(sensitivity) = self.config.get(&self.get_active_window().await).unwrap().settings.get("ANALOG_SENSITIVITY") {
            let polling_rate: u64 = sensitivity.parse::<u64>().expect("Invalid analog sensitivity.");
            while *self.device_is_connected.lock().await {
                {
                    let analog_position = self.analog_position.lock().await;
                    if analog_position[0] != 0 || analog_position[1] != 0 {
                        let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 0, analog_position[0]);
                        let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 1, analog_position[1]);
                        let mut virt_dev = self.virt_dev.lock().await;
                        virt_dev.relative_axes.emit(&[virtual_event_x]).unwrap();
                        virt_dev.relative_axes.emit(&[virtual_event_y]).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(polling_rate)).await;
            }
        } else {
            return
        }
    }

    async fn get_active_window(&self) -> String {
        let active_client = self.current_desktop.clone().unwrap_or(String::from("default"));
        match active_client.as_str() {
            "Hyprland" => {
                let active_window: String = match Client::get_active_async().await.unwrap() {
                    Some(window) => window.class,
                    None => String::from("default")
                };
                if self.config.contains_key(&active_window) {
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
                if self.config.contains_key(&active_window) {
                    active_window
                } else {
                    String::from("default")
                }
            },
            _ => String::from("default")
        }
    }
}

