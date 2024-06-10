use crate::udev_monitor::Client;
use evdev::Key;
use serde;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum Event {
    Axis(Axis),
    Key(Key),
    Hold,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum Axis {
    BTN_DPAD_UP,
    BTN_DPAD_DOWN,
    BTN_DPAD_LEFT,
    BTN_DPAD_RIGHT,
    LSTICK_UP,
    LSTICK_DOWN,
    LSTICK_LEFT,
    LSTICK_RIGHT,
    RSTICK_UP,
    RSTICK_DOWN,
    RSTICK_LEFT,
    RSTICK_RIGHT,
    SCROLL_WHEEL_UP,
    SCROLL_WHEEL_DOWN,
    BTN_TL2,
    BTN_TR2,
    ABS_WHEEL_CW,
    ABS_WHEEL_CCW,
}

impl FromStr for Axis {
    type Err = String;
    fn from_str(s: &str) -> Result<Axis, Self::Err> {
        match s {
            "BTN_DPAD_UP" => Ok(Axis::BTN_DPAD_UP),
            "BTN_DPAD_DOWN" => Ok(Axis::BTN_DPAD_DOWN),
            "BTN_DPAD_LEFT" => Ok(Axis::BTN_DPAD_LEFT),
            "BTN_DPAD_RIGHT" => Ok(Axis::BTN_DPAD_RIGHT),
            "LSTICK_UP" => Ok(Axis::LSTICK_UP),
            "LSTICK_DOWN" => Ok(Axis::LSTICK_DOWN),
            "LSTICK_LEFT" => Ok(Axis::LSTICK_LEFT),
            "LSTICK_RIGHT" => Ok(Axis::LSTICK_RIGHT),
            "RSTICK_UP" => Ok(Axis::RSTICK_UP),
            "RSTICK_DOWN" => Ok(Axis::RSTICK_DOWN),
            "RSTICK_LEFT" => Ok(Axis::RSTICK_LEFT),
            "RSTICK_RIGHT" => Ok(Axis::RSTICK_RIGHT),
            "SCROLL_WHEEL_UP" => Ok(Axis::SCROLL_WHEEL_UP),
            "SCROLL_WHEEL_DOWN" => Ok(Axis::SCROLL_WHEEL_DOWN),
            "BTN_TL2" => Ok(Axis::BTN_TL2),
            "BTN_TR2" => Ok(Axis::BTN_TR2),
            "ABS_WHEEL_CW" => Ok(Axis::ABS_WHEEL_CW),
            "ABS_WHEEL_CCW" => Ok(Axis::ABS_WHEEL_CCW),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Associations {
    pub client: Client,
    pub layout: u16,
}

#[derive(Default, Debug, Clone)]
pub struct Bindings {
    pub remap: HashMap<Event, HashMap<Vec<Event>, Vec<Key>>>,
    pub commands: HashMap<Event, HashMap<Vec<Event>, Vec<String>>>,
}

#[derive(Default, Debug, Clone)]
pub struct MappedModifiers {
    pub default: Vec<Event>,
    pub custom: Vec<Event>,
    pub all: Vec<Event>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RawConfig {
    #[serde(default)]
    pub remap: HashMap<String, Vec<Key>>,
    #[serde(default)]
    pub commands: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

impl RawConfig {
    fn new_from_file(file: &str) -> Self {
        println!(
            "Parsing config file:\n{:?}\n",
            file.rsplit_once("/").unwrap().1
        );
        let file_content: String = std::fs::read_to_string(file).unwrap();
        let raw_config: RawConfig =
            toml::from_str(&file_content).expect("Couldn't parse config file.");
        let remap = raw_config.remap;
        let commands = raw_config.commands;
        let settings = raw_config.settings;
        Self {
            remap,
            commands,
            settings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub associations: Associations,
    pub bindings: Bindings,
    pub settings: HashMap<String, String>,
    pub mapped_modifiers: MappedModifiers,
}

impl Config {
    pub fn new_from_file(file: &str, file_name: String) -> Self {
        let raw_config = RawConfig::new_from_file(file);
        let (bindings, settings, mapped_modifiers) = parse_raw_config(raw_config);
        let associations = Default::default();

        Self {
            name: file_name,
            associations,
            bindings,
            settings,
            mapped_modifiers,
        }
    }

    pub fn new_empty(file_name: String) -> Self {
        Self {
            name: file_name,
            associations: Default::default(),
            bindings: Default::default(),
            settings: Default::default(),
            mapped_modifiers: Default::default(),
        }
    }
}

fn parse_raw_config(raw_config: RawConfig) -> (Bindings, HashMap<String, String>, MappedModifiers) {
    let remap: HashMap<String, Vec<Key>> = raw_config.remap;
    let commands: HashMap<String, Vec<String>> = raw_config.commands;
    let settings: HashMap<String, String> = raw_config.settings;
    let mut bindings: Bindings = Default::default();
    let default_modifiers = vec![
        Event::Key(Key::KEY_LEFTSHIFT),
        Event::Key(Key::KEY_LEFTCTRL),
        Event::Key(Key::KEY_LEFTALT),
        Event::Key(Key::KEY_RIGHTSHIFT),
        Event::Key(Key::KEY_RIGHTCTRL),
        Event::Key(Key::KEY_RIGHTALT),
        Event::Key(Key::KEY_LEFTMETA),
    ];
    let mut mapped_modifiers = MappedModifiers {
        default: default_modifiers,
        custom: Vec::new(),
        all: Vec::new(),
    };
    let custom_modifiers: Vec<Event> = parse_modifiers(&settings, "CUSTOM_MODIFIERS");
    let lstick_activation_modifiers: Vec<Event> =
        parse_modifiers(&settings, "LSTICK_ACTIVATION_MODIFIERS");
    let rstick_activation_modifiers: Vec<Event> =
        parse_modifiers(&settings, "RSTICK_ACTIVATION_MODIFIERS");

    mapped_modifiers.custom.extend(custom_modifiers);
    mapped_modifiers.custom.extend(lstick_activation_modifiers);
    mapped_modifiers.custom.extend(rstick_activation_modifiers);

    for (input, output) in remap.clone() {
        if let Some((mods, event)) = input.rsplit_once("-") {
            let str_modifiers = mods.split("-").collect::<Vec<&str>>();
            let mut modifiers: Vec<Event> = Vec::new();
            for event in str_modifiers.clone() {
                if let Ok(axis) = Axis::from_str(event) {
                    modifiers.push(Event::Axis(axis));
                } else if let Ok(key) = Key::from_str(event) {
                    modifiers.push(Event::Key(key));
                }
            }
            modifiers.sort();
            modifiers.dedup();
            for modifier in &modifiers {
                if !mapped_modifiers.default.contains(&modifier) {
                    mapped_modifiers.custom.push(modifier.clone());
                }
            }
            if str_modifiers[0] == "" {
                modifiers.push(Event::Hold);
            }
            if let Ok(event) = Axis::from_str(event) {
                if !bindings.remap.contains_key(&Event::Axis(event)) {
                    bindings
                        .remap
                        .insert(Event::Axis(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .remap
                        .get_mut(&Event::Axis(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            } else if let Ok(event) = Key::from_str(event) {
                if !bindings.remap.contains_key(&Event::Key(event)) {
                    bindings
                        .remap
                        .insert(Event::Key(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .remap
                        .get_mut(&Event::Key(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            }
        } else {
            let modifiers: Vec<Event> = Vec::new();
            if let Ok(event) = Axis::from_str(input.as_str()) {
                if !bindings.remap.contains_key(&Event::Axis(event)) {
                    bindings
                        .remap
                        .insert(Event::Axis(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .remap
                        .get_mut(&Event::Axis(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            } else if let Ok(event) = Key::from_str(input.as_str()) {
                if !bindings.remap.contains_key(&Event::Key(event)) {
                    bindings
                        .remap
                        .insert(Event::Key(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .remap
                        .get_mut(&Event::Key(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            }
        }
    }

    for (input, output) in commands.clone() {
        if let Some((mods, event)) = input.rsplit_once("-") {
            let str_modifiers = mods.split("-").collect::<Vec<&str>>();
            let mut modifiers: Vec<Event> = Vec::new();
            for event in str_modifiers {
                if let Ok(axis) = Axis::from_str(event) {
                    modifiers.push(Event::Axis(axis));
                } else if let Ok(key) = Key::from_str(event) {
                    modifiers.push(Event::Key(key));
                }
            }
            modifiers.sort();
            modifiers.dedup();
            for modifier in &modifiers {
                if !mapped_modifiers.default.contains(&modifier) {
                    mapped_modifiers.custom.push(modifier.clone());
                }
            }
            if let Ok(event) = Axis::from_str(event) {
                if !bindings.commands.contains_key(&Event::Axis(event)) {
                    bindings
                        .commands
                        .insert(Event::Axis(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .commands
                        .get_mut(&Event::Axis(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            } else if let Ok(event) = Key::from_str(event) {
                if !bindings.commands.contains_key(&Event::Key(event)) {
                    bindings
                        .commands
                        .insert(Event::Key(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .commands
                        .get_mut(&Event::Key(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            }
        } else {
            let modifiers: Vec<Event> = Vec::new();
            if let Ok(event) = Axis::from_str(input.as_str()) {
                if !bindings.commands.contains_key(&Event::Axis(event)) {
                    bindings
                        .commands
                        .insert(Event::Axis(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .commands
                        .get_mut(&Event::Axis(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            } else if let Ok(event) = Key::from_str(input.as_str()) {
                if !bindings.commands.contains_key(&Event::Key(event)) {
                    bindings
                        .commands
                        .insert(Event::Key(event), HashMap::from([(modifiers, output)]));
                } else {
                    bindings
                        .commands
                        .get_mut(&Event::Key(event))
                        .unwrap()
                        .insert(modifiers, output);
                }
            }
        }
    }

    mapped_modifiers.custom.sort();
    mapped_modifiers.custom.dedup();
    mapped_modifiers
        .all
        .extend(mapped_modifiers.default.clone());
    mapped_modifiers.all.extend(mapped_modifiers.custom.clone());
    mapped_modifiers.all.sort();
    mapped_modifiers.all.dedup();

    (bindings, settings, mapped_modifiers)
}

pub fn parse_modifiers(settings: &HashMap<String, String>, parameter: &str) -> Vec<Event> {
    match settings.get(&parameter.to_string()) {
        Some(modifiers) => {
            let mut custom_modifiers = Vec::new();
            let split_modifiers = modifiers.split("-").collect::<Vec<&str>>();
            for modifier in split_modifiers {
                if let Ok(key) = Key::from_str(modifier) {
                    custom_modifiers.push(Event::Key(key));
                } else if let Ok(axis) = Axis::from_str(modifier) {
                    custom_modifiers.push(Event::Axis(axis));
                } else {
                    println!("Invalid value used as modifier in {}, ignoring.", parameter);
                }
            }
            custom_modifiers
        }
        None => Vec::new(),
    }
}
