use std::{collections::HashMap, fmt::Debug, ffi::OsStr, path::Path, sync::Arc};
use evdev::{Device, EventStream, Key, AbsoluteAxisType, EventType, InputEvent};
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_stream::StreamExt;
use tokio_udev;
use tokio;
use serde;
use home;

struct EventReader {
    config: Config,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    analog_position: Arc<Mutex<Vec<i32>>>,
    device_is_connected: Arc<Mutex<bool>>,
}

impl EventReader {
    fn new(config: Config, stream: Arc<Mutex<EventStream>>, virt_dev: Arc<Mutex<VirtualDevices>>) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {position_vector.push(i)};
        let position_vector_mutex = Arc::new(Mutex::new(position_vector));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        Self {
            config: config,
            stream: stream,
            virt_dev: virt_dev,
            analog_position: position_vector_mutex,
            device_is_connected: device_is_connected,
        }
    }
    
    async fn start(&self) {
        let mut stream = self.stream.lock().await;
        let mut analog_mode: &str = "left";
        if let Some(stick) = self.config.settings.get("POINTER_STICK") {
            analog_mode = stick.as_str();
        };
        let mut has_signed_axis_value: &str = "false";
        if let Some(axis_value) = self.config.settings.get("SIGNED_AXIS_VALUE") {
            has_signed_axis_value = axis_value.as_str();
        };
        while let Some(Ok(event)) = stream.next().await {
            match (event.event_type(), AbsoluteAxisType(event.code()), analog_mode) {
                (EventType::KEY, _, _) => {
                    if let Some(event_list) = self.config.keys.get(&Key(event.code())) {
                        self.emit_event(event_list, event.value()).await
                    } else {
                        self.emit_default_event(event).await;
                    }
                },
                (_, AbsoluteAxisType::ABS_HAT0X, _) => {
                    let event_list_option: Option<&Vec<Key>> = match event.value() {
                        -1 => self.config.keys.get(&Key::BTN_DPAD_LEFT),
                        0 => self.config.abs.get(&"NONE_X".to_string()),
                        1 => self.config.keys.get(&Key::BTN_DPAD_RIGHT),
                        _ => self.config.abs.get(&"NONE_X".to_string()),
                    };
                    if let Some(event_list) = event_list_option {
                        self.emit_event(event_list, event.value()).await;
                    } else {
                        println!("Button not set in the config file!");
                    }
                },
                (_, AbsoluteAxisType::ABS_HAT0Y, _) => {
                    let event_list_option: Option<&Vec<Key>> = match event.value() {
                        -1 => self.config.keys.get(&Key::BTN_DPAD_UP),
                        0 => self.config.abs.get(&"NONE_Y".to_string()),
                        1 => self.config.keys.get(&Key::BTN_DPAD_DOWN),
                        _ => self.config.abs.get(&"NONE_Y".to_string()),
                    };
                    if let Some(event_list) = event_list_option {
                        self.emit_event(event_list, event.value()).await;
                    } else {
                        println!("Button not set in the config file!");
                    }
                },
                (EventType::ABSOLUTE, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y, "left") => {
                    let rel_value = self.get_rel_value(&has_signed_axis_value, &event).await;
                    let mut analog_position = self.analog_position.lock().await;
                    analog_position[event.code() as usize] = rel_value;
                },
                (EventType::ABSOLUTE, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY, "right") => {
                    let rel_value = self.get_rel_value(&has_signed_axis_value, &event).await;
                    let mut analog_position = self.analog_position.lock().await;
                    analog_position[(event.code() as usize) -3] = rel_value;
                },
                (EventType::ABSOLUTE, AbsoluteAxisType::ABS_Z, _) => {
                    if let Some(event_list) = self.config.keys.get(&Key::BTN_TL2) {
                        if event.value() == 0 {
                            self.emit_event(event_list, event.value()).await
                        } else {
                            self.emit_event(event_list, 1).await
                        };
                    } else {
                        println!("Button not set in the config file!");
                    };
                },
                (EventType::ABSOLUTE, AbsoluteAxisType::ABS_RZ, _) => {
                    if let Some(event_list) = self.config.keys.get(&Key::BTN_TR2) {
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

    async fn cursor_loop(&self) {
        if let Some(sensitivity) = self.config.settings.get("ANALOG_SENSITIVITY") {
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
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Config {
    #[serde(skip)]
    name: String,
    keys: HashMap<Key, Vec<Key>>,
    settings: HashMap<String, String>,
    #[serde(skip)]
    abs: HashMap<String, Vec<Key>>,
}

impl Config {
    fn new_from_file(file: &str, file_name: String) -> Self {
        println!("Parsing config file at {:?}", file);
        let file_content: String = std::fs::read_to_string(file).unwrap();
        let config: Config = toml::from_str(&file_content).expect("Couldn't parse config file.");
        let keys: HashMap<Key, Vec<Key>> = config.keys;
        let mut abs: HashMap<String, Vec<Key>> = HashMap::new();
        let mut pad_horizontal: Vec<Key> = keys.get(&Key::BTN_DPAD_LEFT).unwrap_or(&Vec::new()).clone();
        pad_horizontal.extend(keys.get(&Key::BTN_DPAD_RIGHT).unwrap_or(&Vec::new()));
        let mut pad_vertical: Vec<Key> = keys.get(&Key::BTN_DPAD_UP).unwrap_or(&Vec::new()).clone();
        pad_vertical.extend(keys.get(&Key::BTN_DPAD_DOWN).unwrap_or(&Vec::new()));
        abs.insert("NONE_X".to_string(), pad_horizontal);
        abs.insert("NONE_Y".to_string(), pad_vertical);
        let settings: HashMap<String, String> = config.settings;
        Self {
            name: file_name,
            keys: keys,
            settings: settings,
            abs: abs,
        }
    }
}

struct VirtualDevices {
    keys: VirtualDevice,
    relative_axes: VirtualDevice,
}

impl VirtualDevices {
    fn new(keys: VirtualDevice, relative_axes: VirtualDevice) -> Self {
        Self {
            keys: keys,
            relative_axes: relative_axes,
        }
    }
}

async fn create_new_reader(device: String, config: Config) {
    let stream: Arc<Mutex<EventStream>> = Arc::new(Mutex::new(get_event_stream(Path::new(&device), config.clone())));
    let virt_dev: Arc<Mutex<VirtualDevices>> = Arc::new(Mutex::new(new_virtual_devices()));
    let reader = EventReader::new(config.clone(), stream, virt_dev);
    println!("Mapped device detected at {}, reading events.", device);
    tokio::join!(
        reader.start(),
        reader.cursor_loop(),
    );
    println!("Disconnected device at {}.", device);
}

async fn start_monitoring_udev(config_files: Vec<Config>, mut tasks: Vec<JoinHandle<()>>) {
    launch_tasks(&config_files, &mut tasks);
    let mut monitor = tokio_udev::AsyncMonitorSocket::new(
        tokio_udev::MonitorBuilder::new().unwrap()
        .match_subsystem(OsStr::new("input")).unwrap()
        .listen().unwrap()
        ).unwrap();
    while let Some(Ok(event)) = monitor.next().await {
        if is_mapped(&event.device(), &config_files) {
            println!("Reinitializing...");
            for task in &tasks {
                task.abort();
            }
            tasks.clear();
            launch_tasks(&config_files, &mut tasks)
        }
    }
}

fn get_event_stream(path: &Path, config: Config) -> EventStream {
    let mut device: Device = Device::open(path).expect("Couldn't open device path.");
    if config.settings.get("GRAB_DEVICE").expect("No GRAB_DEVICE setting specified, this device will be ignored.") == &"true".to_string() {
        device.grab().unwrap();
    };
    let stream: EventStream = device.into_event_stream().unwrap();
    return stream
}

fn new_virtual_devices() -> VirtualDevices {
    let mut key_capabilities = evdev::AttributeSet::new();
    for i in 1..334 {key_capabilities.insert(Key(i));};
    let mut rel_capabilities = evdev::AttributeSet::new();
    for i in 0..13 {rel_capabilities.insert(evdev::RelativeAxisType(i));};
    let keys_builder = VirtualDeviceBuilder::new().unwrap()
        .name("Makima Virtual Keyboard/Mouse")
        .with_keys(&key_capabilities).unwrap();
    let rel_builder = VirtualDeviceBuilder::new().unwrap()
        .name("Makima Virtual Pointer")
        .with_relative_axes(&rel_capabilities).unwrap();
    let virtual_device_keys = keys_builder.build().unwrap();
    let virtual_device_rel = rel_builder.build().unwrap();
    let virtual_devices = VirtualDevices::new(virtual_device_keys, virtual_device_rel);
    return virtual_devices;
}

fn launch_tasks(config_files: &Vec<Config>, tasks: &mut Vec<JoinHandle<()>>) {
    let devices: evdev::EnumerateDevices = evdev::enumerate();
    for device in devices {
        for config in config_files {
            if config.name == device.1.name().unwrap().to_string() {
                tasks.push(tokio::spawn(create_new_reader(device.0.as_path().to_str().unwrap().to_string(), config.clone())));
            }
        }
    }
}

fn is_mapped(udev_device: &tokio_udev::Device, config_files: &Vec<Config>) -> bool {
    match udev_device.devnode() {
        Some(devnode) => {
            let evdev_devices: evdev::EnumerateDevices = evdev::enumerate();
            for evdev_device in evdev_devices {
                for config in config_files {
                    if config.name == evdev_device.1.name().unwrap().to_string()
                    && devnode.to_path_buf() == evdev_device.0 {
                        return true
                    }
                }
            }
        }
        _ => return false
    }
    return false
}

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
