use std::{collections::{HashMap, BTreeMap}, sync::Arc, path::Path, env};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use evdev::{Device, EventStream, Key};
use crate::Config;
use crate::event_reader::EventReader;


pub async fn start_monitoring_udev(config_files: Vec<Config>, mut tasks: Vec<JoinHandle<()>>) {
    launch_tasks(&config_files, &mut tasks);
    let mut monitor = tokio_udev::AsyncMonitorSocket::new(
        tokio_udev::MonitorBuilder::new().unwrap()
        .match_subsystem(std::ffi::OsStr::new("input")).unwrap()
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

pub fn launch_tasks(config_files: &Vec<Config>, tasks: &mut Vec<JoinHandle<()>>) {
    let modifiers: Arc<Mutex<BTreeMap<Key, i32>>> = Arc::new (
        Mutex::new (
            BTreeMap::from ([
                (Key::KEY_LEFTSHIFT, 0),
                (Key::KEY_LEFTCTRL, 0),
                (Key::KEY_LEFTALT, 0),
                (Key::KEY_RIGHTSHIFT, 0),
                (Key::KEY_RIGHTCTRL, 0),
                (Key::KEY_RIGHTALT, 0),
                (Key::KEY_LEFTMETA, 0)
            ])
        )
    );
    let current_desktop: Option<String> = match (env::var("XDG_SESSION_TYPE"), env::var("XDG_CURRENT_DESKTOP")) {
        (Ok(session), Ok(desktop)) if session == "wayland".to_string() && vec!["Hyprland".to_string(), "sway".to_string()].contains(&desktop)  => {
            println!(">> Running on {}, active window detection enabled.\n", desktop);
            Option::Some(desktop)
        },
        (Ok(session), Ok(desktop)) if session == "wayland".to_string() => {
            println!(">> Unsupported compositor: {}, won't be able to change bindings according to active window.\n
                    Currently supported desktops: Hyprland, Sway.\n", desktop);
            Option::None
        },
        (Ok(session), _) if session == "x11".to_string() => {
            println!(">> Running on X11, active window detection enabled.");
            Option::Some("x11".to_string())
        },
        (Ok(session), Err(_)) if session == "wayland".to_string() => {
            println!(">> Unable to retrieve the current desktop based on XDG_CURRENT_DESKTOP env var.\n
                    Won't be able to change bindings according to the active window.\n");
            Option::None
        },
        (Err(_), _) => {
            println!(">> Unable to retrieve the session type based on XDG_SESSION_TYPE env var.\n
                    Won't be able to change bindings according to the active window.\n");
            Option::None
        },
        _ => Option::None
    };
    let devices: evdev::EnumerateDevices = evdev::enumerate();
    for device in devices {
        let mut config_map: HashMap<String, Config> = HashMap::new();
        for config in config_files {
            let split_config_name = config.name.split("::").collect::<Vec<&str>>();
            let associated_device_name = split_config_name[0];
            if associated_device_name == device.1.name().unwrap().replace("/", "") {
                let window_class = if split_config_name.len() == 1 {
                    String::from("default")
                } else {
                    split_config_name[1].to_string()
                };
                config_map.insert(window_class, config.clone());
            };
        }
        let event_device = device.0.as_path().to_str().unwrap().to_string();
        if !config_map.is_empty() {
            let stream = Arc::new(Mutex::new(get_event_stream(Path::new(&event_device), config_map.clone())));
            let reader = EventReader::new(config_map.clone(), stream, modifiers.clone(), current_desktop.clone());
            tasks.push(tokio::spawn(start_reader(reader)));
        }
    }
}

pub async fn start_reader(reader: EventReader) {
    reader.start().await;
}

pub fn get_event_stream(path: &Path, config: HashMap<String, Config>) -> EventStream {
    let mut device: Device = Device::open(path).expect("Couldn't open device path.");
    if config.get("default")
        .unwrap()
        .settings
        .get("GRAB_DEVICE")
        .expect("No GRAB_DEVICE setting specified, this device will be ignored.") == &"true".to_string()
    {
        device.grab().unwrap();
    };

    let stream: EventStream = device.into_event_stream().unwrap();
    return stream
}

pub fn is_mapped(udev_device: &tokio_udev::Device, config_files: &Vec<Config>) -> bool {
    match udev_device.devnode() {
        Some(devnode) => {
            let evdev_devices: evdev::EnumerateDevices = evdev::enumerate();
            for evdev_device in evdev_devices {
                for config in config_files {
                    if config.name.contains(&evdev_device.1.name().unwrap().to_string())
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

