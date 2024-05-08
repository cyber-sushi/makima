use std::{collections::HashMap, sync::Arc, option::Option, process::{Command, Stdio}};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use fork::{fork, setsid, Fork};
use evdev::{EventStream, Key, RelativeAxisType, AbsoluteAxisType, EventType, InputEvent};
use crate::virtual_devices::VirtualDevices;
use crate::Config;
use crate::active_client::*;
use crate::udev_monitor::Environment;

struct Stick {
    function: String,
    sensitivity: u64,
    deadzone: i32,
}

struct Settings {
    lstick: Stick,
    rstick: Stick,
    axis_16_bit: bool,
}

pub struct EventReader {
    config: HashMap<String, Config>,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    lstick_position: Arc<Mutex<Vec<i32>>>,
    rstick_position: Arc<Mutex<Vec<i32>>>,
    modifiers: Arc<Mutex<Vec<Key>>>,
    modifier_was_activated: Arc<Mutex<bool>>,
    device_is_connected: Arc<Mutex<bool>>,
    current_desktop: Option<String>,
    environment: Environment,
    settings: Settings,
}

impl EventReader {
    pub fn new(
        config: HashMap<String, Config>,
        stream: Arc<Mutex<EventStream>>,
        modifiers: Arc<Mutex<Vec<Key>>>,
        modifier_was_activated: Arc<Mutex<bool>>,
        environment: Environment,
        current_desktop: Option<String>,
    ) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {position_vector.push(i)};
        let lstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let rstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        let virt_dev = Arc::new(Mutex::new(VirtualDevices::new()));
        let lstick_function = config.get(&"default".to_string()).unwrap()
            .settings.get("LSTICK").unwrap_or(&"cursor".to_string()).to_string();
        let lstick_sensitivity: u64 = config.get(&"default".to_string()).unwrap()
            .settings.get("LSTICK_SENSITIVITY").unwrap_or(&"0".to_string()).parse::<u64>().expect("Invalid value for LSTICK_SENSITIVITY, please use an integer value >= 0");
        let lstick_deadzone: i32 = config.get(&"default".to_string()).unwrap()
            .settings.get("LSTICK_DEADZONE").unwrap_or(&"5".to_string()).parse::<i32>().expect("Invalid value for LSTICK_DEADZONE, please use an integer between 0 and 128.");
        let lstick = Stick {
            function: lstick_function,
            sensitivity: lstick_sensitivity,
            deadzone: lstick_deadzone,
        };

        let rstick_function: String = config.get(&"default".to_string()).unwrap()
            .settings.get("RSTICK").unwrap_or(&"scroll".to_string()).to_string();
        let rstick_sensitivity: u64 = config.get(&"default".to_string()).unwrap()
            .settings.get("RSTICK_SENSITIVITY").unwrap_or(&"0".to_string()).parse::<u64>().expect("Invalid value for RSTICK_SENSITIVITY, please use an integer value >= 0");
        let rstick_deadzone: i32 = config.get(&"default".to_string()).unwrap()
            .settings.get("RSTICK_DEADZONE").unwrap_or(&"5".to_string()).parse::<i32>().expect("Invalid value for RSTICK_DEADZONE, please use an integer between 0 and 128.");
        let rstick = Stick {
            function: rstick_function,
            sensitivity: rstick_sensitivity,
            deadzone: rstick_deadzone,
        };

        let axis_16_bit: bool = config.get(&"default".to_string()).unwrap()
            .settings.get("16_BIT_AXIS").unwrap_or(&"false".to_string()).parse().expect("16_BIT_AXIS can only be true or false.");

        let settings = Settings {
            lstick,
            rstick,
            axis_16_bit,
        };
        Self {
            config,
            stream,
            virt_dev,
            lstick_position,
            rstick_position,
            modifiers,
            modifier_was_activated,
            device_is_connected,
            current_desktop,
            environment,
            settings,
        }
    }

    pub async fn start(&self) {
        println!("{:?} detected, reading events.\n", self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap().name);
        tokio::join!(
            self.event_loop(),
            self.cursor_loop(),
            self.scroll_loop(),
        );
    }

    pub async fn event_loop(&self) {
        let mut lstick_values = HashMap::from([("x", 0), ("y", 0)]);
        let mut rstick_values = HashMap::from([("x", 0), ("y", 0)]);
        let mut ltrigger_value = 0;
        let mut rtrigger_value = 0;
        let mut stream = self.stream.lock().await;
        while let Some(Ok(event)) = stream.next().await {
            match (event.event_type(), RelativeAxisType(event.code()), AbsoluteAxisType(event.code())) {
                (EventType::KEY, _, _) => {
                    self.convert_key_events(event).await;
                },
                (_, RelativeAxisType::REL_WHEEL | RelativeAxisType::REL_WHEEL_HI_RES, _) => {
                    let event_string_option: Option<String> = match event.value() {
                        -1 => Option::Some("SCROLL_WHEEL_DOWN".to_string()),
                        1 => Option::Some("SCROLL_WHEEL_UP".to_string()),
                        _ => Option::None,
                    };
                    if let Some(event_string) = event_string_option {
                        self.convert_axis_events(event, &event_string, true).await;
                    }
                },
                (_, _, AbsoluteAxisType::ABS_HAT0X) => {
                    let event_string: String = match event.value() {
                        -1 => "BTN_DPAD_LEFT".to_string(),
                        0 => "BTN_DPAD_X".to_string(),
                        1 => "BTN_DPAD_RIGHT".to_string(),
                        _ => "BTN_DPAD_X".to_string(),
                    };
                    self.convert_axis_events(event, &event_string, false).await;
                },
                (_, _, AbsoluteAxisType::ABS_HAT0Y) => {
                    let event_string: String = match event.value() {
                        -1 => "BTN_DPAD_UP".to_string(),
                        0 => "BTN_DPAD_Y".to_string(),
                        1 => "BTN_DPAD_DOWN".to_string(),
                        _ => "BTN_DPAD_Y".to_string(),
                    };
                    self.convert_axis_events(event, &event_string, false).await;
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y) => {
                    if ["cursor", "scroll"].contains(&self.settings.lstick.function.as_str()) {
                        let axis_value = self.get_axis_value(&event, &self.settings.lstick.deadzone).await;
                        let mut lstick_position = self.lstick_position.lock().await;
                        lstick_position[event.code() as usize] = axis_value;
                    } else if self.settings.lstick.function.as_str() == "bind" {
                        let axis_value = self.get_axis_value(&event, &self.settings.lstick.deadzone).await;
                        let clamped_value = if axis_value < 0 { -1 }
                            else if axis_value > 0 { 1 }
                            else { 0 };
                        let axis = if AbsoluteAxisType(event.code()) == AbsoluteAxisType::ABS_X { "x" }
                            else if AbsoluteAxisType(event.code()) == AbsoluteAxisType::ABS_Y { "y" }
                            else { "none" };
                        let event_string_option: Option<String> = match clamped_value {
                            -1 if axis == "x" && lstick_values.get("x").unwrap() != &-1 => {
                                lstick_values.insert("x", -1);
                                Option::Some("LSTICK_LEFT".to_string())
                            },
                            -1 if axis == "y" && lstick_values.get("y").unwrap() != &-1 => {
                                lstick_values.insert("y", -1);
                                Option::Some("LSTICK_UP".to_string())
                            },
                            0 if axis == "x" && lstick_values.get("x").unwrap() != &0 => {
                                lstick_values.insert("x", 0);
                                Option::Some("LSTICK_X".to_string())
                            },
                            0 if axis == "y" && lstick_values.get("y").unwrap() != &0 => {
                                lstick_values.insert("y", 0);
                                Option::Some("LSTICK_Y".to_string())
                            },
                            1 if axis == "x" && lstick_values.get("x").unwrap() != &1 => {
                                lstick_values.insert("x", 1);
                                Option::Some("LSTICK_RIGHT".to_string())
                            },
                            1 if axis == "y" && lstick_values.get("y").unwrap() != &1 => {
                                lstick_values.insert("y", 1);
                                Option::Some("LSTICK_DOWN".to_string())
                            },
                            _ => Option::None,
                        };
                        if let Some(event_string) = event_string_option {
                            let clamped_event = InputEvent::new_now(event.event_type(), event.code(), clamped_value);
                            self.convert_axis_events(clamped_event, &event_string, false).await;
                        }
                    } else {
                        self.emit_default_event(event).await;
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY) => {
                    if ["cursor", "scroll"].contains(&self.settings.rstick.function.as_str()) {
                        let axis_value = self.get_axis_value(&event, &self.settings.rstick.deadzone).await;
                        let mut rstick_position = self.rstick_position.lock().await;
                        rstick_position[event.code() as usize -3] = axis_value;
                    } else if self.settings.rstick.function.as_str() == "bind" {
                        let axis_value = self.get_axis_value(&event, &self.settings.rstick.deadzone).await;
                        let clamped_value = if axis_value < 0 { -1 }
                            else if axis_value > 0 { 1 }
                            else { 0 };
                        let axis = if AbsoluteAxisType(event.code()) == AbsoluteAxisType::ABS_RX { "x" }
                            else if AbsoluteAxisType(event.code()) == AbsoluteAxisType::ABS_RY { "y" }
                            else { "none" };
                        let event_string_option: Option<String> = match clamped_value {
                            -1 if axis == "x" && rstick_values.get("x").unwrap() != &-1 => {
                                rstick_values.insert("x", -1);
                                Option::Some("RSTICK_LEFT".to_string())
                            },
                            -1 if axis == "y" && rstick_values.get("y").unwrap() != &-1 => {
                                rstick_values.insert("y", -1);
                                Option::Some("RSTICK_UP".to_string())
                            },
                            0 if axis == "x" && rstick_values.get("x").unwrap() != &0 => {
                                rstick_values.insert("x", 0);
                                Option::Some("RSTICK_X".to_string())
                            },
                            0 if axis == "y" && rstick_values.get("y").unwrap() != &0 => {
                                rstick_values.insert("y", 0);
                                Option::Some("RSTICK_Y".to_string())
                            },
                            1 if axis == "x" && rstick_values.get("x").unwrap() != &1 => {
                                rstick_values.insert("x", 1);
                                Option::Some("RSTICK_RIGHT".to_string())
                            },
                            1 if axis == "y" && rstick_values.get("y").unwrap() != &1 => {
                                rstick_values.insert("y", 1);
                                Option::Some("RSTICK_DOWN".to_string())
                            },
                            _ => Option::None,
                        };
                        if let Some(event_string) = event_string_option {
                            let clamped_event = InputEvent::new_now(event.event_type(), event.code(), clamped_value);
                            self.convert_axis_events(clamped_event, &event_string, false).await;
                        }
                    } else {
                        self.emit_default_event(event).await;
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_Z) => {
                    let clamped_value = if event.value() > 0 { 1 } else { 0 };
                    if clamped_value == 1 && ltrigger_value == 0 {
                        ltrigger_value = 1;
                        let clamped_event = InputEvent::new_now(event.event_type(), event.code(), clamped_value);
                        self.convert_axis_events(clamped_event, &"BTN_TL2".to_string(), false).await;
                    } else if clamped_value == 0 {
                        ltrigger_value = 0;
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RZ) => {
                    let clamped_value = if event.value() > 0 { 1 } else { 0 };
                    if clamped_value == 1 && rtrigger_value == 0 {
                        rtrigger_value = 1;
                        let clamped_event = InputEvent::new_now(event.event_type(), event.code(), clamped_value);
                        self.convert_axis_events(clamped_event, &"BTN_TR2".to_string(), false).await;
                    } else if clamped_value == 0 {
                        rtrigger_value = 0;
                    }
                },
                _ => {self.emit_default_event(event).await;}
            }
        }
        let mut device_is_connected = self.device_is_connected.lock().await;
        *device_is_connected = false;
        println!("Disconnected device \"{}\".\n", self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap().name);
    }

    async fn convert_key_events(&self, event: InputEvent) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let modifiers = self.modifiers.lock().await.clone();
        if let Some(event_hashmap) = path.combinations.keys.get(&Key(event.code())) {
            if let Some(event_list) = event_hashmap.get(&modifiers) {
                self.emit_event_without_modifiers(event_list, &modifiers, event.value()).await;
                return
            }
        }
        if let Some(command_hashmap) = path.combinations.keys_sh.get(&Key(event.code())) {
            if let Some(command_list) = command_hashmap.get(&modifiers) {
                if event.value() == 1 {self.spawn_subprocess(command_list).await};
                return
            }
        }
        if let Some(event_list) = path.bindings.keys.get(&Key(event.code())) {
            self.emit_event(event_list, event.value()).await;
            return
        }
        if let Some(command_list) = path.bindings.keys_sh.get(&Key(event.code())) {
            if event.value() == 1 {self.spawn_subprocess(command_list).await};
            return
        }
        self.emit_default_event(event).await;
    }
    
    async fn convert_axis_events(&self, event: InputEvent, event_string: &String, send_zero: bool) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let modifiers = self.modifiers.lock().await.clone();
        if let Some(event_hashmap) = path.combinations.axis.get(event_string) {
            if let Some(event_list) = event_hashmap.get(&modifiers) {
                self.emit_event_without_modifiers(event_list, &modifiers, event.value().abs()).await;
                if send_zero {
                    self.emit_event_without_modifiers(event_list, &modifiers, 0).await;
                }
                return
            }
        }
        if let Some(command_hashmap) = path.combinations.axis_sh.get(event_string) {
            if let Some(command_list) = command_hashmap.get(&modifiers) {
                if event.value().abs() == 1 {self.spawn_subprocess(command_list).await};
                return
            }
        }
        if let Some(event_list) = path.bindings.axis.get(event_string) {
            self.emit_event(event_list, event.value().abs()).await;
            if send_zero {
                self.emit_event_without_modifiers(event_list, &modifiers, 0).await;
            }
            return
        }
        if let Some(command_list) = path.bindings.axis_sh.get(event_string) {
            if event.value().abs() == 1 {self.spawn_subprocess(command_list).await};
            return
        }
        self.emit_default_event(event).await;
    }

    async fn emit_event(&self, event_list: &Vec<Key>, value: i32) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut virt_dev = self.virt_dev.lock().await;
        let modifiers = self.modifiers.lock().await.clone();
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        let released_keys: Vec<Key> = self.released_keys(&modifiers).await;
        for key in released_keys {
            self.toggle_modifiers(key, 0).await;
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
        for key in event_list {
            self.toggle_modifiers(*key, value).await;
            if path.mapped_modifiers.custom.contains(&key) {
                if value == 0 && !*modifier_was_activated {
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 1);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    *modifier_was_activated = true;
                } else if value == 1 {
                    *modifier_was_activated = false;
                }
            } else {
                let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), value);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                *modifier_was_activated = true;
            }
        }
    }
    
    async fn emit_default_event(&self, event: InputEvent) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        match event.event_type() {
            EventType::KEY => {
                let modifiers = self.modifiers.lock().await.clone();
                let released_keys: Vec<Key> = self.released_keys(&modifiers).await;
                for key in released_keys {
                    self.toggle_modifiers(key, 0).await;
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap()
                }
                self.toggle_modifiers(Key(event.code()), event.value()).await;
                if path.mapped_modifiers.custom.contains(&Key(event.code())) {
                    if event.value() == 0 && !*modifier_was_activated {
                        let virtual_event: InputEvent = InputEvent::new_now(event.event_type(), event.code(), 1);
                        virt_dev.keys.emit(&[virtual_event]).unwrap();
                        let virtual_event: InputEvent = InputEvent::new_now(event.event_type(), event.code(), 0);
                        virt_dev.keys.emit(&[virtual_event]).unwrap();
                        *modifier_was_activated = true;
                    } else if event.value() == 1 {
                        *modifier_was_activated = false;
                    }
                } else {
                    virt_dev.keys.emit(&[event]).unwrap();
                    *modifier_was_activated = true;
                }
            },
            EventType::RELATIVE => {
                virt_dev.axis.emit(&[event]).unwrap();
                *modifier_was_activated = true;
            },
            _ => {}
        }
    }
    
    async fn emit_event_without_modifiers(&self, event_list: &Vec<Key>, modifiers: &Vec<Key>, value: i32) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        for key in modifiers {
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
        for key in event_list {
            if path.mapped_modifiers.custom.contains(&key) {
                if value == 0 && !*modifier_was_activated {
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 1);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    *modifier_was_activated = true;
                } else if value == 1 {
                    *modifier_was_activated = false;
                }
            } else {
                let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), value);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                *modifier_was_activated = true;
            }
        }
    }

    async fn spawn_subprocess(&self, command_list: &Vec<String>) {
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        *modifier_was_activated = true;
        match &self.environment.user {
            Ok(user) if user == &"root".to_string() => {
                match &self.environment.sudo_user {
                    Ok(sudo_user) => {
                        for command in command_list {
                            Command::new("sh")
                                .arg("-c")
                                .arg(format!("env runuser {} -c {}", sudo_user.as_str(), command))
                                .spawn()
                                .expect("Failed to run command.");
                        }
                    },
                    _ => {}
                }
            },
            Ok(_) => {
                for command in command_list {
                    match fork() {
                        Ok(Fork::Child) => {
                            setsid().unwrap();
                            Command::new("sh")
                                .arg("-c")
                                .arg(command)
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn()
                                .expect("Failed to run command.");
                            std::process::exit(0);
                        },
                        Ok(Fork::Parent(_)) => (),
                        Err(_) => std::process::exit(1),
                    }
                }
            }
            Err(_) => {},
        }
    }

    async fn get_axis_value(&self, event: &InputEvent, deadzone: &i32) -> i32 {
        let distance_from_center: i32 = match self.settings.axis_16_bit {
            false => (event.value() as i32 - 128) * 200,
            _ => event.value() as i32
        };
        if distance_from_center.abs() <= deadzone * 200 {
            0
        } else {
            (distance_from_center + 2000 - 1) / 2000
        }
    }
    
    async fn toggle_modifiers(&self, key: Key, value: i32) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut modifiers = self.modifiers.lock().await;
        if path.mapped_modifiers.all.contains(&key) {
            match value {
                1 => {
                    modifiers.push(key);
                    modifiers.sort();
                    modifiers.dedup();
                },
                0 => modifiers.retain(|&x| x != key),
                _ => {},
            }
        }
    }

    async fn released_keys(&self, modifiers: &Vec<Key>) -> Vec<Key> {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut released_keys: Vec<Key> = Vec::new();
        for (_key, hashmap) in path.combinations.keys.iter() {
            if let Some(event_list) = hashmap.get(modifiers) {
                released_keys.extend(event_list);
            }
        }
        for (_key, hashmap) in path.combinations.axis.iter() {
            if let Some(event_list) = hashmap.get(modifiers) {
                released_keys.extend(event_list);
            }
        }
        released_keys
    }

    pub async fn cursor_loop(&self) {
        let (cursor, sensitivity) = if self.settings.lstick.function.as_str() == "cursor" {
            ("left", self.settings.lstick.sensitivity)
        } else if self.settings.rstick.function.as_str() == "cursor" {
            ("right", self.settings.rstick.sensitivity)
        } else {
            ("disabled", 0)
        };
        if sensitivity != 0 {
            while *self.device_is_connected.lock().await {
                {
                    let stick_position = if cursor == "left" {
                        self.lstick_position.lock().await
                    } else if cursor == "right" {
                        self.rstick_position.lock().await
                    } else {
                        break
                    };
                    if stick_position[0] != 0 || stick_position[1] != 0 {
                        let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 0, stick_position[0]);
                        let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 1, stick_position[1]);
                        let mut virt_dev = self.virt_dev.lock().await;
                        virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                        virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return
        }
    }

    pub async fn scroll_loop(&self) {
        let (scroll, sensitivity) = if self.settings.lstick.function.as_str() == "scroll" {
            ("left", self.settings.lstick.sensitivity)
        } else if self.settings.rstick.function.as_str() == "scroll" {
            ("right", self.settings.rstick.sensitivity)
        } else {
            ("disabled", 0)
        };
        if sensitivity != 0 {
            while *self.device_is_connected.lock().await {
                {
                    let stick_position = if scroll == "left" {
                        self.lstick_position.lock().await
                    } else if scroll == "right" {
                        self.rstick_position.lock().await
                    } else {
                        break
                    };
                    if stick_position[0] != 0 || stick_position[1] != 0 {
                        let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 12, stick_position[0]);
                        let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 11, stick_position[1]);
                        let mut virt_dev = self.virt_dev.lock().await;
                        virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                        virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return
        }
    }
}
