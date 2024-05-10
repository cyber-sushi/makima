use std::{collections::HashMap, sync::Arc, path::Path, process::Command, env};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use evdev::{Device, EventStream, Key};
use crate::Config;
use crate::event_reader::EventReader;


#[derive(Clone)]
pub struct Environment {
    pub user: Result<String, env::VarError>,
    pub sudo_user: Result<String, env::VarError>,
    pub session_address: Result<String, env::VarError>,
}

pub async fn start_monitoring_udev(config_files: Vec<Config>, mut tasks: Vec<JoinHandle<()>>) {
    launch_tasks(&config_files, &mut tasks);
    let mut monitor = tokio_udev::AsyncMonitorSocket::new (
        tokio_udev::MonitorBuilder::new().unwrap()
        .match_subsystem(std::ffi::OsStr::new("input")).unwrap()
        .listen().unwrap()
        ).unwrap();
    while let Some(Ok(event)) = monitor.next().await {
        if is_mapped(&event.device(), &config_files) {
            println!("---------------------\n\nReinitializing...\n");
            for task in &tasks {
                task.abort();
            }
            tasks.clear();
            launch_tasks(&config_files, &mut tasks)
        }
    }
}

pub fn launch_tasks(config_files: &Vec<Config>, tasks: &mut Vec<JoinHandle<()>>) {
    let modifiers: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Default::default()));
    let modifier_was_activated: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    match env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(_) => {
            let command = Command::new("sh").arg("-c").arg("systemctl --user show-environment").output().unwrap();
            let vars = std::str::from_utf8(command.stdout.as_slice()).unwrap().split("\n").collect::<Vec<&str>>();
            for var in vars {
                if var != "" && !var.contains("DBUS_SESSION_BUS_ADDRESS") {
                    let split_var = var.split("=").collect::<Vec<&str>>();
                    env::set_var(split_var[0], split_var[1]);
                }
            }
            true
        },
        Err(_) => {
            println!("Warning: unable to inherit user environment.\n\
                    Launch Makima with 'sudo -E makima' or add the DBUS_SESSION_BUS_ADDRESS env var to your systemd unit if you're running it through systemd.\n");
            false
        },
    };
    let environment = Environment {
        user: env::var("USER"),
        sudo_user: env::var("SUDO_USER"),
        session_address: env::var("DBUS_SESSION_BUS_ADDRESS"),
    };
    let mut session_var = "WAYLAND_DISPLAY";
    if let Err(env::VarError::NotPresent) = env::var(session_var) {
        session_var = "XDG_SESSION_TYPE";
    }
    let current_desktop: Option<String> = match (env::var(session_var), env::var("XDG_CURRENT_DESKTOP")) {
        (Ok(session), Ok(desktop)) if session.contains("wayland") && vec!["Hyprland".to_string(), "sway".to_string()].contains(&desktop)  => {
            println!("Running on {}, active window tracking enabled.", desktop);
            Option::Some(desktop)
        },
        (Ok(session), Ok(desktop)) if session.contains("wayland") => {
            println!("Warning: unsupported compositor: {}, won't be able to change bindings according to active window.\n\
                    Currently supported desktops: Hyprland, Sway, X11.\n", desktop);
            Option::None
        },
        (Ok(session), _) if session == "x11".to_string() => {
            println!("Running on X11, active window tracking enabled.");
            Option::Some("x11".to_string())
        },
        (Ok(session), Err(_)) if session.contains("wayland") => {
            println!("Warning: unable to retrieve the current desktop based on XDG_CURRENT_DESKTOP env var.\n\
                    Won't be able to change bindings according to the active window.\n");
            Option::None
        },
        (Err(_), _) => {
            println!("Warning: unable to retrieve the session type based on XDG_SESSION_TYPE or WAYLAND_DISPLAY env vars.\n\
                    Won't be able to change bindings according to the active window.\n");
            Option::None
        },
        _ => Option::None
    };
    let user_has_access = match Command::new("groups").output() {
        Ok(groups) if std::str::from_utf8(&groups.stdout.as_slice()).unwrap().contains("input") => {
            println!("Evdev permissions available.\nScanning for event devices with a matching config file...\n");
            true
        },
        Ok(groups) if std::str::from_utf8(&groups.stdout.as_slice()).unwrap().contains("root") => {
            println!("Root permissions available.\nScanning for event devices with a matching config file...\n");
            true
        }
        Ok(_) => {
            println!("Warning: user has no access to event devices, Makima might not be able to detect all connected devices.\n\
                    Note: Run Makima with 'sudo -E makima' or as a system service. Refer to the docs for more info. Continuing...\n");
            false
        },
        Err(_) => {
            println!("Unable to determine if user has access to event devices. Continuing...\n");
            false
        },
    };
    let devices: evdev::EnumerateDevices = evdev::enumerate();
    let mut devices_found = 0;
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
        if config_map.len() > 0 && !config_map.contains_key(&"default".to_string()) {
            config_map.insert("default".to_string(), Config::new_empty(device.1.name().unwrap().replace("/", "")));
        }
        let event_device = device.0.as_path().to_str().unwrap().to_string();
        if !config_map.is_empty() {
            let stream = Arc::new(Mutex::new(get_event_stream(Path::new(&event_device), config_map.clone())));
            let reader = EventReader::new(config_map.clone(), stream, modifiers.clone(), modifier_was_activated.clone(), environment.clone(), current_desktop.clone());
            tasks.push(tokio::spawn(start_reader(reader)));
            devices_found += 1
        }
    }
    if devices_found == 0 && !user_has_access {
        println!("No matching devices found.\nNote: make sure that your user has access to event devices.\n");
    } else if devices_found == 0 && user_has_access {
        println!("No matching devices found.\nNote: double-check that your device and its respective config file have the same name, as reported by `evtest`.\n");
    }
}

pub async fn start_reader(reader: EventReader) {
    reader.start().await;
}

pub fn get_event_stream(path: &Path, config: HashMap<String, Config>) -> EventStream {
    let mut device: Device = Device::open(path).expect("Couldn't open device path.");
	match config.get("default").unwrap().settings.get("GRAB_DEVICE") {
		Some(value) => {
			if value == &true.to_string() {
				device.grab().expect("Unable to grab device. Is another instance of Makima running?")
			}
		}
		None => device.grab().expect("Unable to grab device. Is another instance of Makima running?")
	}
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

