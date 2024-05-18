use std::{collections::HashMap, sync::Arc, option::Option, process::{Command, Stdio}};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use fork::{fork, Fork, setsid};
use evdev::{EventStream, Key, RelativeAxisType, AbsoluteAxisType, EventType, InputEvent};
use crate::virtual_devices::VirtualDevices;
use crate::Config;
use crate::config::{Event, Axis, parse_modifiers};
use crate::active_client::*;
use crate::udev_monitor::{Environment, Client};

struct Stick {
    function: String,
    sensitivity: u64,
    deadzone: i32,
    activation_modifiers: Vec<Event>,
}

struct Settings {
    lstick: Stick,
    rstick: Stick,
    axis_16_bit: bool,
}

pub struct EventReader {
    config: HashMap<Client, Config>,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    lstick_position: Arc<Mutex<Vec<i32>>>,
    rstick_position: Arc<Mutex<Vec<i32>>>,
    modifiers: Arc<Mutex<Vec<Event>>>,
    modifier_was_activated: Arc<Mutex<bool>>,
    device_is_connected: Arc<Mutex<bool>>,
    environment: Environment,
    settings: Settings,
}

impl EventReader {
    pub fn new (
        config: HashMap<Client, Config>,
        stream: Arc<Mutex<EventStream>>,
        modifiers: Arc<Mutex<Vec<Event>>>,
        modifier_was_activated: Arc<Mutex<bool>>,
        environment: Environment,
    ) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {position_vector.push(i)};
        let lstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let rstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        let virt_dev = Arc::new(Mutex::new(VirtualDevices::new()));
        let lstick_function = config.get(&Client::Default).unwrap()
            .settings.get("LSTICK").unwrap_or(&"cursor".to_string()).to_string();
        let lstick_sensitivity: u64 = config.get(&Client::Default).unwrap()
            .settings.get("LSTICK_SENSITIVITY").unwrap_or(&"0".to_string()).parse::<u64>().expect("Invalid value for LSTICK_SENSITIVITY, please use an integer value >= 0");
        let lstick_deadzone: i32 = config.get(&Client::Default).unwrap()
            .settings.get("LSTICK_DEADZONE").unwrap_or(&"5".to_string()).parse::<i32>().expect("Invalid value for LSTICK_DEADZONE, please use an integer between 0 and 128.");
        let lstick_activation_modifiers: Vec<Event> = parse_modifiers(&config.get(&Client::Default).unwrap().settings, "LSTICK_ACTIVATION_MODIFIERS");
        let lstick = Stick {
            function: lstick_function,
            sensitivity: lstick_sensitivity,
            deadzone: lstick_deadzone,
            activation_modifiers: lstick_activation_modifiers,
        };

        let rstick_function: String = config.get(&Client::Default).unwrap()
            .settings.get("RSTICK").unwrap_or(&"scroll".to_string()).to_string();
        let rstick_sensitivity: u64 = config.get(&Client::Default).unwrap()
            .settings.get("RSTICK_SENSITIVITY").unwrap_or(&"0".to_string()).parse::<u64>().expect("Invalid value for RSTICK_SENSITIVITY, please use an integer value >= 0");
        let rstick_deadzone: i32 = config.get(&Client::Default).unwrap()
            .settings.get("RSTICK_DEADZONE").unwrap_or(&"5".to_string()).parse::<i32>().expect("Invalid value for RSTICK_DEADZONE, please use an integer between 0 and 128.");
        let rstick_activation_modifiers: Vec<Event> = parse_modifiers(&config.get(&Client::Default).unwrap().settings, "RSTICK_ACTIVATION_MODIFIERS");
        let rstick = Stick {
            function: rstick_function,
            sensitivity: rstick_sensitivity,
            deadzone: rstick_deadzone,
            activation_modifiers: rstick_activation_modifiers,
        };

        let axis_16_bit: bool = config.get(&Client::Default).unwrap()
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
            environment,
            settings,
        }
    }

    pub async fn start(&self) {
        println!("{:?} detected, reading events.\n", self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap().name);
        tokio::join!(
            self.event_loop(),
            self.cursor_loop(),
            self.scroll_loop(),
        );
    }

    pub async fn event_loop(&self) {
        let (mut dpad_values, mut lstick_values, mut rstick_values, mut triggers_values) = ((0, 0), (0, 0), (0, 0), (0, 0));
        let mut stream = self.stream.lock().await;
        while let Some(Ok(event)) = stream.next().await {
            match (event.event_type(), RelativeAxisType(event.code()), AbsoluteAxisType(event.code())) {
                (EventType::KEY, _, _) => {
                    self.convert_event(event, Event::Key(Key(event.code())), event.value()).await;
                },
                (_, RelativeAxisType::REL_WHEEL | RelativeAxisType::REL_WHEEL_HI_RES, _) => {
                    match event.value() {
                        -1 => {
                            self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_DOWN), 1).await;
                            self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_DOWN), 0).await;
                        },
                        1 => {
                            self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_UP), 1).await;
                            self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_UP), 0).await;
                        },
                        _ => {}
                    }
                },
                (_, _, AbsoluteAxisType::ABS_HAT0X) => {
                    match event.value() {
                        -1 => {
                                self.convert_event(event, Event::Axis(Axis::BTN_DPAD_LEFT), 1).await;
                                dpad_values.0 = -1;
                        },
                        1 => {
                                self.convert_event(event, Event::Axis(Axis::BTN_DPAD_RIGHT), 1).await;
                                dpad_values.0 = 1;
                        },
                        0 => {
                            match dpad_values.0 {
                                -1 => self.convert_event(event, Event::Axis(Axis::BTN_DPAD_LEFT), 0).await,
                                1 => self.convert_event(event, Event::Axis(Axis::BTN_DPAD_RIGHT), 0).await,
                                _ => {},
                            }
                            dpad_values.0 = 0;
                        },
                        _ => {}
                    };
                },
                (_, _, AbsoluteAxisType::ABS_HAT0Y) => {
                    match event.value() {
                        -1 => {
                                self.convert_event(event, Event::Axis(Axis::BTN_DPAD_UP), 1).await;
                                dpad_values.1 = -1;
                        },
                        1 => {
                                self.convert_event(event, Event::Axis(Axis::BTN_DPAD_DOWN), 1).await;
                                dpad_values.1 = 1;
                        },
                        0 => {
                            match dpad_values.1 {
                                -1 => self.convert_event(event, Event::Axis(Axis::BTN_DPAD_UP), 0).await,
                                1 => self.convert_event(event, Event::Axis(Axis::BTN_DPAD_DOWN), 0).await,
                                _ => {},
                            }
                            dpad_values.1 = 0;
                        },
                        _ => {}
                    };
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y) => {
                    match self.settings.lstick.function.as_str() {
                        "cursor" | "scroll" => {
                            let axis_value = self.get_axis_value(&event, &self.settings.lstick.deadzone).await;
                            let mut lstick_position = self.lstick_position.lock().await;
                            lstick_position[event.code() as usize] = axis_value;
                        },
                        "bind" => {
                            let axis_value = self.get_axis_value(&event, &self.settings.lstick.deadzone).await;
                            let clamped_value =
                                if axis_value < 0 { -1 }
                                else if axis_value > 0 { 1 }
                                else { 0 };
                            match AbsoluteAxisType(event.code()) {
                                AbsoluteAxisType::ABS_Y => {
                                    match clamped_value {
                                        -1 if lstick_values.1 != -1 => {
                                            self.convert_event(event, Event::Axis(Axis::LSTICK_UP), 1).await;
                                            lstick_values.1 = -1
                                        },
                                        1 if lstick_values.1 != 1 => {
                                            self.convert_event(event, Event::Axis(Axis::LSTICK_DOWN), 1).await;
                                            lstick_values.1 = 1
                                        },
                                        0 => if lstick_values.1 != 0 {
                                            match lstick_values.1 {
                                                -1 => self.convert_event(event, Event::Axis(Axis::LSTICK_UP), 0).await,
                                                1 => self.convert_event(event, Event::Axis(Axis::LSTICK_DOWN), 0).await,
                                                _ => {},
                                            }
                                            lstick_values.1 = 0;
                                        },
                                        _ => {},
                                    }
                                },
                                AbsoluteAxisType::ABS_X => {
                                    match clamped_value {
                                        -1 if lstick_values.0 != -1 => {
                                            self.convert_event(event, Event::Axis(Axis::LSTICK_LEFT), 1).await;
                                            lstick_values.0 = -1
                                        },
                                        1 => if lstick_values.0 != 1 {
                                            self.convert_event(event, Event::Axis(Axis::LSTICK_RIGHT), 1).await;
                                            lstick_values.0 = 1
                                        },
                                        0 => if lstick_values.0 != 0 {
                                            match lstick_values.0 {
                                                -1 => self.convert_event(event, Event::Axis(Axis::LSTICK_LEFT), 0).await,
                                                1 => self.convert_event(event, Event::Axis(Axis::LSTICK_RIGHT), 0).await,
                                                _ => {},
                                            }
                                            lstick_values.0 = 0;
                                        },
                                        _ => {},
                                    }
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY) => {
                    match self.settings.rstick.function.as_str() {
                        "cursor" | "scroll" => {
                            let axis_value = self.get_axis_value(&event, &self.settings.rstick.deadzone).await;
                            let mut rstick_position = self.rstick_position.lock().await;
                            rstick_position[event.code() as usize -3] = axis_value;
                        },
                        "bind" => {
                            let axis_value = self.get_axis_value(&event, &self.settings.rstick.deadzone).await;
                            let clamped_value = if axis_value < 0 { -1 }
                                else if axis_value > 0 { 1 }
                                else { 0 };
                            match AbsoluteAxisType(event.code()) {
                                AbsoluteAxisType::ABS_RY => {
                                    match clamped_value {
                                        -1 => if rstick_values.1 != -1 {
                                            self.convert_event(event, Event::Axis(Axis::RSTICK_UP), 1).await;
                                            rstick_values.1 = -1
                                        },
                                        1 => if rstick_values.1 != 1 {
                                            self.convert_event(event, Event::Axis(Axis::RSTICK_DOWN), 1).await;
                                            rstick_values.1 = 1
                                        },
                                        0 => if rstick_values.1 != 0 {
                                            match rstick_values.1 {
                                                -1 => self.convert_event(event, Event::Axis(Axis::RSTICK_UP), 0).await,
                                                1 => self.convert_event(event, Event::Axis(Axis::RSTICK_DOWN), 0).await,
                                                _ => {},
                                            }
                                            rstick_values.1 = 0;
                                        },
                                        _ => {},
                                    }
                                },
                                AbsoluteAxisType::ABS_RX => {
                                    match clamped_value {
                                        -1 if rstick_values.0 != -1 => {
                                            self.convert_event(event, Event::Axis(Axis::RSTICK_LEFT), 1).await;
                                            rstick_values.0 = -1
                                        },
                                        1 => if rstick_values.0 != 1 {
                                            self.convert_event(event, Event::Axis(Axis::RSTICK_RIGHT), 1).await;
                                            rstick_values.0 = 1
                                        },
                                        0 => if rstick_values.0 != 0 {
                                            match rstick_values.0 {
                                                -1 => self.convert_event(event, Event::Axis(Axis::RSTICK_LEFT), 0).await,
                                                1 => self.convert_event(event, Event::Axis(Axis::RSTICK_RIGHT), 0).await,
                                                _ => {},
                                            }
                                            rstick_values.0 = 0;
                                        },
                                        _ => {},
                                    }
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_Z) => {
                    match (event.value(), triggers_values.0) {
                        (0, 1) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TL2), 0).await;
                            triggers_values.0 = 0;
                        },
                        (_, 0) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TL2), 1).await;
                            triggers_values.0 = 1;
                        },
                        _ => {},
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RZ) => {
                    match (event.value(), triggers_values.1) {
                        (0, 1) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TR2), 0).await;
                            triggers_values.1 = 0;
                        },
                        (_, 0) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TR2), 1).await;
                            triggers_values.1 = 1;
                        },
                        _ => {},
                    }
                },
                _ => self.emit_default_event(event).await,
            }
        }
        let mut device_is_connected = self.device_is_connected.lock().await;
        *device_is_connected = false;
        println!("Disconnected device \"{}\".\n", self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap().name);
    }

    async fn convert_event(&self, default_event: InputEvent, event: Event, value: i32) {
        let path = self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap();
        let modifiers = self.modifiers.lock().await.clone();
        if let Some(map) = path.bindings.remap.get(&event) {
            if let Some(event_list) = map.get(&modifiers) {
                self.emit_event(event_list, value, &modifiers).await;
                return
            }
        }
        if let Some(map) = path.bindings.commands.get(&event) {
            if let Some(command_list) = map.get(&modifiers) {
                if value == 1 {self.spawn_subprocess(command_list).await};
                return
            }
        }
        self.emit_nonmapped_event(default_event, event, value, &modifiers).await;
    }

    async fn emit_event(&self, event_list: &Vec<Key>, value: i32, modifiers: &Vec<Event>) {
        let path = self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap();
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        if modifiers.is_empty() {
            let released_keys: Vec<Key> = self.released_keys(&modifiers).await;
            for key in released_keys {
                self.toggle_modifiers(Event::Key(key), 0).await;
                let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
            }
        } else {
            for key in modifiers.iter() {
                if let Event::Key(key) = key {
                    let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                }
            }
        }
        for key in event_list {
            if modifiers.is_empty() {
                self.toggle_modifiers(Event::Key(*key), value).await;
            }
            if path.mapped_modifiers.custom.contains(&Event::Key(*key)) {
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

    async fn emit_nonmapped_event(&self, default_event: InputEvent, event: Event, value: i32, modifiers: &Vec<Event>) {
        let path = self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap();
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        let released_keys: Vec<Key> = self.released_keys(&modifiers).await;
        for key in released_keys {
            self.toggle_modifiers(Event::Key(key), 0).await;
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap()
        }
        self.toggle_modifiers(event, value).await;
        if path.mapped_modifiers.custom.contains(&event) {
            if value == 0 && !*modifier_was_activated {
                let virtual_event: InputEvent = InputEvent::new_now(default_event.event_type(), default_event.code(), 1);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                let virtual_event: InputEvent = InputEvent::new_now(default_event.event_type(), default_event.code(), 0);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                *modifier_was_activated = true;
            } else if value == 1 {
                *modifier_was_activated = false;
            }
        } else {
            *modifier_was_activated = true;
            match default_event.event_type() {
                EventType::KEY => {
                    virt_dev.keys.emit(&[default_event]).unwrap();
                },
                EventType::RELATIVE => {
                    if value == 1 {
                        virt_dev.axis.emit(&[default_event]).unwrap();
                    }
                },
                _ => {},
            }
        }
    }

    async fn emit_default_event(&self, event: InputEvent) {
        match event.event_type() {
            EventType::KEY => {
                let mut virt_dev = self.virt_dev.lock().await;
                virt_dev.keys.emit(&[event]).unwrap();
            },
            EventType::RELATIVE => {
                let mut virt_dev = self.virt_dev.lock().await;
                virt_dev.axis.emit(&[event]).unwrap();
            },
            _ => {},
        }
    }

    async fn spawn_subprocess(&self, command_list: &Vec<String>) {
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        *modifier_was_activated = true;
        let (user, running_as_root) =
            if let Ok(sudo_user) = &self.environment.sudo_user {
                (Option::Some(sudo_user), true)
            }
            else if let Ok(user) = &self.environment.user {
                (Option::Some(user), false)
            }
            else {
                (Option::None, false)
            };
        if let Some(user) = user {
            for command in command_list {
                let cmd = if running_as_root {
                    let cmd = format!("runuser {} -c '{}'", user, command);
                    cmd
                }
                else {
                    command.clone()
                };
                match fork() {
                    Ok(Fork::Child) => {
                        match fork() {
                            Ok(Fork::Child) => {
                                setsid().unwrap();
                                Command::new("sh")
                                    .arg("-c")
                                    .arg(cmd)
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .spawn()
                                    .unwrap();
                                std::process::exit(0);
                            }
                            Ok(Fork::Parent(_)) => std::process::exit(0),
                            Err(_) => std::process::exit(1),
                        }
                    }
                    Ok(Fork::Parent(_)) => (),
                    Err(_) => std::process::exit(1),
                }
            }
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
    
    async fn toggle_modifiers(&self, modifier: Event, value: i32) {
        let path = self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap();
        let mut modifiers = self.modifiers.lock().await;
        if path.mapped_modifiers.all.contains(&modifier) {
            match value {
                1 => {
                    modifiers.push(modifier);
                    modifiers.sort();
                    modifiers.dedup();
                },
                0 => modifiers.retain(|&x| x != modifier),
                _ => {},
            }
        }
    }

    async fn released_keys(&self, modifiers: &Vec<Event>) -> Vec<Key> {
        let path = self.config.get(&get_active_window(&self.environment.server, &self.config).await).unwrap();
        let mut released_keys: Vec<Key> = Vec::new();
        for (_key, hashmap) in path.bindings.remap.iter() {
            if let Some(event_list) = hashmap.get(modifiers) {
                released_keys.extend(event_list);
            }
        }
        released_keys
    }

    pub async fn cursor_loop(&self) {
        let (cursor, sensitivity, activation_modifiers) = if self.settings.lstick.function.as_str() == "cursor" {
            ("left", self.settings.lstick.sensitivity, self.settings.lstick.activation_modifiers.clone())
        } else if self.settings.rstick.function.as_str() == "cursor" {
            ("right", self.settings.rstick.sensitivity, self.settings.rstick.activation_modifiers.clone())
        } else {
            ("disabled", 0, vec![])
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
                        let modifiers = self.modifiers.lock().await;
                        if activation_modifiers.len() == 0 || activation_modifiers == *modifiers {
                            let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 0, stick_position[0]);
                            let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 1, stick_position[1]);
                            let mut virt_dev = self.virt_dev.lock().await;
                            virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                            virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return
        }
    }

    pub async fn scroll_loop(&self) {
        let (scroll, sensitivity, activation_modifiers) = if self.settings.lstick.function.as_str() == "scroll" {
            ("left", self.settings.lstick.sensitivity, self.settings.lstick.activation_modifiers.clone())
        } else if self.settings.rstick.function.as_str() == "scroll" {
            ("right", self.settings.rstick.sensitivity, self.settings.rstick.activation_modifiers.clone())
        } else {
            ("disabled", 0, vec![])
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
                        let modifiers = self.modifiers.lock().await;
                        if activation_modifiers.len() == 0 || activation_modifiers == *modifiers {
                            let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 12, stick_position[0]);
                            let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 11, stick_position[1]);
                            let mut virt_dev = self.virt_dev.lock().await;
                            virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                            virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return
        }
    }
}
