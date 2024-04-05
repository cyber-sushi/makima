use std::{collections::{HashMap, BTreeMap}, sync::Arc, option::Option};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use evdev::{EventStream, Key, RelativeAxisType, AbsoluteAxisType, EventType, InputEvent};
use crate::virtual_devices::VirtualDevices;
use crate::Config;
use crate::active_client::*;


pub struct EventReader {
    config: HashMap<String, Config>,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    cursor_analog_position: Arc<Mutex<Vec<i32>>>,
    scroll_analog_position: Arc<Mutex<Vec<i32>>>,
    modifiers: Arc<Mutex<BTreeMap<Key, i32>>>,
    device_is_connected: Arc<Mutex<bool>>,
    current_desktop: Option<String>,
}

impl EventReader {
    pub fn new(
        config: HashMap<String, Config>,
        stream: Arc<Mutex<EventStream>>,
        modifiers: Arc<Mutex<BTreeMap<Key, i32>>>,
        current_desktop: Option<String>,
    ) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {position_vector.push(i)};
        let cursor_position_vector_mutex = Arc::new(Mutex::new(position_vector.clone()));
        let scroll_position_vector_mutex = Arc::new(Mutex::new(position_vector.clone()));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        let virt_dev = Arc::new(Mutex::new(VirtualDevices::new()));
        Self {
            config: config,
            stream: stream,
            virt_dev: virt_dev,
            cursor_analog_position: cursor_position_vector_mutex,
            scroll_analog_position: scroll_position_vector_mutex,
            modifiers: modifiers,
            device_is_connected: device_is_connected,
            current_desktop: current_desktop,
        }
    }

    pub async fn start(&self) {
        println!("{:?} detected, reading events.", self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap().name);
        tokio::join!(
            self.event_loop(),
            self.cursor_loop(),
            self.scroll_loop(),
        );
    }

    pub async fn event_loop(&self) {
        let mut stream = self.stream.lock().await;
        let mut cursor_analog_mode: &str = "left";
        if let Some(stick) = self.config.get(&"default".to_string()).unwrap().settings.get("CURSOR_STICK") {
            cursor_analog_mode = stick.as_str();
        }
        let mut scroll_analog_mode: &str = "right";
        if let Some(stick) = self.config.get(&"default".to_string()).unwrap().settings.get("SCROLL_STICK") {
            scroll_analog_mode = stick.as_str();
        }
        let mut has_signed_axis_value: &str = "false";
        if let Some(axis_value) = self.config.get(&"default".to_string()).unwrap().settings.get("SIGNED_AXIS_VALUE") {
            has_signed_axis_value = axis_value.as_str();
        }
        let mut deadzone: i32 = 5;
        if let Some(deadzone_str) = self.config.get(&"default".to_string()).unwrap().settings.get("DEADZONE") {
            deadzone = deadzone_str.parse::<i32>().expect("Invalid value for DEADZONE, please use an integer between 0 and 128.");
        }
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
                        self.convert_axis_events(event, &event_string, true, false).await;
                    } else {
                        self.emit_default_event(event).await;
                    }
                },
                (_, _, AbsoluteAxisType::ABS_HAT0X) => {
                    let event_string: String = match event.value() {
                        -1 => "BTN_DPAD_LEFT".to_string(),
                        0 => "NONE_X".to_string(),
                        1 => "BTN_DPAD_RIGHT".to_string(),
                        _ => "NONE_X".to_string(),
                    };
                    self.convert_axis_events(event, &event_string, false, false).await;
                },
                (_, _, AbsoluteAxisType::ABS_HAT0Y) => {
                    let event_string: String = match event.value() {
                        -1 => "BTN_DPAD_UP".to_string(),
                        0 => "NONE_Y".to_string(),
                        1 => "BTN_DPAD_DOWN".to_string(),
                        _ => "NONE_Y".to_string(),
                    };
                    self.convert_axis_events(event, &event_string, false, false).await;
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y) => {
                    if cursor_analog_mode == "left" {
                        let axis_value = self.get_axis_value(&has_signed_axis_value, &event, deadzone).await;
                        let mut cursor_analog_position = self.cursor_analog_position.lock().await;
                        cursor_analog_position[event.code() as usize] = axis_value;
                    } else if scroll_analog_mode == "left" {
                        let axis_value = self.get_axis_value(&has_signed_axis_value, &event, deadzone).await;
                        let mut scroll_analog_position = self.scroll_analog_position.lock().await;
                        scroll_analog_position[event.code() as usize] = axis_value;
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY) => {
                    if cursor_analog_mode == "right" {
                        let axis_value = self.get_axis_value(&has_signed_axis_value, &event, deadzone).await;
                        let mut cursor_analog_position = self.cursor_analog_position.lock().await;
                        cursor_analog_position[event.code() as usize -3] = axis_value;
                    } else if scroll_analog_mode == "right" {
                        let axis_value = self.get_axis_value(&has_signed_axis_value, &event, deadzone).await;
                        let mut scroll_analog_position = self.scroll_analog_position.lock().await;
                        scroll_analog_position[event.code() as usize -3] = axis_value;
                    }
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_Z) => {
                    self.convert_axis_events(event, &"BTN_TL2".to_string(), false, true).await;
                },
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RZ) => {
                    self.convert_axis_events(event, &"BTN_TR2".to_string(), false, true).await;
                },
                _ => {self.emit_default_event(event).await;}
            }
        }
        let mut device_is_connected = self.device_is_connected.lock().await;
        *device_is_connected = false;
        println!("Disconnected device {}.", self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap().name);
    }

    async fn convert_key_events(&self, event: InputEvent) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let modifiers = self.modifiers.lock().await.clone();
        if let Some(event_hashmap) = path.modifiers.keys.get(&modifiers) {
            if let Some(event_list) = event_hashmap.get(&Key(event.code())) {
                self.emit_event_without_modifiers(event_list, &modifiers, event.value()).await;
                return
            }
        }
        if let Some(event_list) = path.bindings.keys.get(&Key(event.code())) {
            self.emit_event(event_list, event.value()).await;
        } else {
            self.emit_default_event(event).await;
        }
    }
    
    async fn convert_axis_events(&self, event: InputEvent, event_string: &String, send_zero: bool, clamp_value: bool) {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let modifiers = self.modifiers.lock().await.clone();
        let value = {
            if clamp_value && event.value() > 1 {
                1 
            } else {
                event.value()
            }
        };
        if let Some(event_hashmap) = path.modifiers.axis.get(&modifiers) {
            if let Some(event_list) = event_hashmap.get(event_string) {
                self.emit_event_without_modifiers(event_list, &modifiers, value).await;
                if send_zero {
                    self.emit_event_without_modifiers(event_list, &modifiers, 0).await;
                }
                return
            }
        }
        if let Some(event_list) = path.bindings.axis.get(event_string) {
            self.emit_event(event_list, value).await;
            if send_zero {
                self.emit_event_without_modifiers(event_list, &modifiers, 0).await;
            }
        } else {
            self.emit_default_event(event).await;
        }
    }

    async fn emit_event(&self, event_list: &Vec<Key>, value: i32) {
        let mut virt_dev = self.virt_dev.lock().await;
        let modifiers = self.modifiers.lock().await.clone();
        let released_keys: Vec<Key> = self.released_keys(&modifiers).await;
        for key in released_keys {
            self.toggle_modifiers(key, 0).await;
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
        for key in event_list {
            self.toggle_modifiers(*key, value).await;
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), value);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
    }
    
    async fn emit_default_event(&self, event: InputEvent) {
        let mut virt_dev = self.virt_dev.lock().await;
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
                virt_dev.keys.emit(&[event]).unwrap();
            },
            EventType::RELATIVE => virt_dev.axis.emit(&[event]).unwrap(),
            _ => {}
        }
    }
    
    async fn emit_event_without_modifiers(&self, event_list: &Vec<Key>, modifiers: &BTreeMap<Key, i32>, value: i32) {
        let modifiers_list = modifiers.iter()
            .filter(|(_key, value)| value == &&1)
            .collect::<HashMap<&Key, &i32>>()
            .into_keys().copied()
            .collect::<Vec<Key>>();
        let mut virt_dev = self.virt_dev.lock().await;
        for key in modifiers_list {
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
        for key in event_list {
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), value);
            virt_dev.keys.emit(&[virtual_event]).unwrap();
        }
    }
    
    async fn get_axis_value(&self, has_signed_axis_value: &str, event: &InputEvent, deadzone: i32) -> i32 {
        let distance_from_center: i32 = match &has_signed_axis_value {
            &"false" => (event.value() as i32 - 128) * 200,
            _ => event.value() as i32
        };
        if distance_from_center.abs() <= deadzone * 200 {
            0
        } else {
            (distance_from_center + 2000 - 1) / 2000
        }
    }
    
    async fn toggle_modifiers(&self, key: Key, value: i32) {
        let mut modifiers = self.modifiers.lock().await;
        if modifiers.contains_key(&key) && vec![0, 1].contains(&value) {
            modifiers.insert(key, value).unwrap();
        }
    }
    
    async fn released_keys(&self, modifiers: &BTreeMap<Key, i32>) -> Vec<Key> {
        let path = self.config.get(&get_active_window(&self.current_desktop, &self.config).await).unwrap();
        let mut released_keys: Vec<Key> = Vec::new();
        if let Some(event_hashmap) = path.modifiers.keys.get(&modifiers) {
            event_hashmap.iter().for_each(|(_modifiers, event_list)| released_keys.extend(event_list));
        }
        if let Some(event_hashmap) = path.modifiers.axis.get(&modifiers) {
            event_hashmap.iter().for_each(|(_modifiers, event_list)| released_keys.extend(event_list));
        }
        released_keys
    }

    pub async fn cursor_loop(&self) {
        if let Some(sensitivity) = self.config.get(&"default".to_string()).unwrap().settings.get("CURSOR_SENSITIVITY") {
            let polling_rate: u64 = sensitivity.parse::<u64>().expect("Invalid cursor sensitivity.");
            while *self.device_is_connected.lock().await {
                {
                    let cursor_analog_position = self.cursor_analog_position.lock().await;
                    if cursor_analog_position[0] != 0 || cursor_analog_position[1] != 0 {
                        let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 0, cursor_analog_position[0]);
                        let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 1, cursor_analog_position[1]);
                        let mut virt_dev = self.virt_dev.lock().await;
                        virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                        virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(polling_rate)).await;
            }
        } else {
            return
        }
    }
    pub async fn scroll_loop(&self) {
        if let Some(sensitivity) = self.config.get(&"default".to_string()).unwrap().settings.get("SCROLL_SENSITIVITY") {
            let polling_rate: u64 = sensitivity.parse::<u64>().expect("Invalid scroll sensitivity.");
            while *self.device_is_connected.lock().await {
                {
                    let scroll_analog_position = self.scroll_analog_position.lock().await;
                    if scroll_analog_position[0] != 0 || scroll_analog_position[1] != 0 {
                        let virtual_event_x: InputEvent = InputEvent::new_now(EventType::RELATIVE, 12, scroll_analog_position[0]);
                        let virtual_event_y: InputEvent = InputEvent::new_now(EventType::RELATIVE, 11, scroll_analog_position[1]);
                        let mut virt_dev = self.virt_dev.lock().await;
                        virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                        virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(polling_rate)).await;
            }
        } else {
            return
        }
    }
}

