use std::collections::HashMap;
use std::str::FromStr;
use evdev::Key;
use serde;


#[derive(Default, Debug, Clone)]
pub struct Bindings {
    pub keys: HashMap<Key, Vec<Key>>,
    pub axis: HashMap<String, Vec<Key>>,
    pub keys_sh: HashMap<Key, Vec<String>>,
    pub axis_sh: HashMap<String, Vec<String>>,
}

#[derive(Default, Debug, Clone)]
pub struct Combinations {
    pub keys: HashMap<Key, HashMap<Vec<Key>, Vec<Key>>>,
    pub axis: HashMap<String, HashMap<Vec<Key>, Vec<Key>>>,
    pub keys_sh: HashMap<Key, HashMap<Vec<Key>, Vec<String>>>,
    pub axis_sh: HashMap<String, HashMap<Vec<Key>, Vec<String>>>,
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

#[derive(Default, Debug, Clone)]
pub struct MappedModifiers {
    pub default: Vec<Key>,
    pub custom: Vec<Key>,
    pub all: Vec<Key>,
}

impl RawConfig {
    fn new_from_file(file: &str) -> Self {
        println!("Parsing config file:\n{:?}\n", file.rsplit_once("/").unwrap().1);
        let file_content: String = std::fs::read_to_string(file).unwrap();
        let raw_config: RawConfig = toml::from_str(&file_content)
            .expect("Couldn't parse config file.");
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
    pub bindings: Bindings,
    pub combinations: Combinations,
    pub settings: HashMap<String, String>,
    pub mapped_modifiers: MappedModifiers,
}

impl Config {
    pub fn new_from_file(file: &str, file_name: String) -> Self {
        let raw_config = RawConfig::new_from_file(file);
        let (bindings, combinations, settings, mapped_modifiers) = parse_raw_config(raw_config);
        let bindings: Bindings = merge_axis_bindings(bindings);

        Self {
            name: file_name,
            bindings,
            combinations,
            settings,
            mapped_modifiers,
        }
    }

    pub fn new_empty(file_name: String) -> Self {
        Self {
            name: file_name,
            bindings: Default::default(),
            combinations: Default::default(),
            settings: Default::default(),
            mapped_modifiers: Default::default(),
        }
    }
}

fn parse_raw_config(raw_config: RawConfig) -> (Bindings, Combinations, HashMap<String, String>, MappedModifiers) {
    let remap: HashMap<String, Vec<Key>> = raw_config.remap;
    let commands: HashMap<String, Vec<String>> = raw_config.commands;
    let settings: HashMap<String, String> = raw_config.settings;
    let mut bindings: Bindings = Default::default();
    let mut combinations: Combinations = Default::default();
    let default_modifiers = vec![
        Key::KEY_LEFTSHIFT,
        Key::KEY_LEFTCTRL,
        Key::KEY_LEFTALT,
        Key::KEY_RIGHTSHIFT,
        Key::KEY_RIGHTCTRL,
        Key::KEY_RIGHTALT,
        Key::KEY_LEFTMETA,
    ];
    let mut mapped_modifiers = MappedModifiers {
        default: default_modifiers.clone(),
        custom: Vec::new(),
        all: Vec::new(),
    };
    let custom_modifiers: Vec<Key> = match settings.get(&"CUSTOM_MODIFIERS".to_string()) {
        Some(modifiers) => {
            modifiers.split("-").collect::<Vec<&str>>().iter()
            .map(|key_str| Key::from_str(key_str).expect("Invalid KEY value used as modifier in CUSTOM_MODIFIERS.")).collect()
        },
        None => Vec::new(),
    };
    mapped_modifiers.custom.extend(custom_modifiers);

    let abs = [
        "BTN_DPAD_UP",
        "BTN_DPAD_DOWN",
        "BTN_DPAD_LEFT",
        "BTN_DPAD_RIGHT",
        "LSTICK_UP",
        "LSTICK_DOWN",
        "LSTICK_LEFT",
        "LSTICK_RIGHT",
        "RSTICK_UP",
        "RSTICK_DOWN",
        "RSTICK_LEFT",
        "RSTICK_RIGHT",
        "SCROLL_WHEEL_UP",
        "SCROLL_WHEEL_DOWN",
        "BTN_TL2",
        "BTN_TR2",
    ];

    for (input, output) in remap.clone().into_iter() {
        if input.contains("-") {
            let (mods, key) = input.rsplit_once("-").unwrap();
            let mut modifiers: Vec<Key> = mods.split("-").collect::<Vec<&str>>().iter().map(|key_str| Key::from_str(key_str).expect("Invalid KEY value used as modifier.")).collect();
            modifiers.sort();
            modifiers.dedup();
            for modifier in &modifiers {
                if !mapped_modifiers.default.contains(&modifier) {
                    mapped_modifiers.custom.push(modifier.clone());
                }
            }
            if abs.contains(&key) {
                if !combinations.axis.contains_key(&key.to_string()) {
                    combinations.axis.insert(key.to_string(), HashMap::from([(modifiers , output)]));
                } else {
                    combinations.axis.get_mut(key).unwrap().insert(modifiers, output);
                }
            } else {
                if !combinations.keys.contains_key(&Key::from_str(key).unwrap()) {
                    combinations.keys.insert(Key::from_str(key).unwrap(), HashMap::from([(modifiers, output)]));
                } else {
                    combinations.keys.get_mut(&Key::from_str(key).unwrap()).unwrap().insert(modifiers, output);
                }
            }
        } else {
            if abs.contains(&input.as_str()) {
                bindings.axis.insert(input, output);
            } else {
                bindings.keys.insert(Key::from_str(input.as_str()).expect("Invalid KEY value used for rebinding."), output);
            }
        }
    }

    for (input, output) in commands.clone().into_iter() {
        if input.contains("-") {
            let (mods, key) = input.rsplit_once("-").unwrap();
            let mut modifiers: Vec<Key> = mods.split("-").collect::<Vec<&str>>().iter().map(|key_str| Key::from_str(key_str).expect("Invalid KEY value used as modifier.")).collect();
            modifiers.sort();
            modifiers.dedup();
            for modifier in &modifiers {
                if !mapped_modifiers.default.contains(&modifier) {
                    mapped_modifiers.custom.push(modifier.clone());
                }
            }
            if abs.contains(&key) {
                if !combinations.axis_sh.contains_key(&key.to_string()) {
                    combinations.axis_sh.insert(key.to_string(), HashMap::from([(modifiers, output)]));
                } else {
                    combinations.axis_sh.get_mut(key).unwrap().insert(modifiers, output);
                }
            } else {
                if !combinations.keys_sh.contains_key(&Key::from_str(key).unwrap()) {
                    combinations.keys_sh.insert(Key::from_str(key).unwrap(), HashMap::from([(modifiers, output)]));
                } else {
                    combinations.keys_sh.get_mut(&Key::from_str(key).unwrap()).unwrap().insert(modifiers, output);
                }
            }
        } else {
            if abs.contains(&input.as_str()) {
                bindings.axis_sh.insert(input, output);
            } else {
                bindings.keys_sh.insert(Key::from_str(input.as_str()).expect("Invalid KEY value used for rebinding."), output);
            }
        }
    }

    mapped_modifiers.custom.sort();
    mapped_modifiers.custom.dedup();
    mapped_modifiers.all.extend(mapped_modifiers.default.clone());
    mapped_modifiers.all.extend(mapped_modifiers.custom.clone());
    mapped_modifiers.all.sort();
    mapped_modifiers.all.dedup();

    (bindings, combinations, settings, mapped_modifiers)
}

fn merge_axis_bindings(mut bindings: Bindings) -> Bindings {
        let mut pad_x: Vec<Key> = bindings.axis.get("BTN_DPAD_LEFT")
            .unwrap_or(&Vec::new()).clone();
        pad_x.extend(bindings.axis.get("BTN_DPAD_RIGHT")
            .unwrap_or(&Vec::new()));
        let mut pad_y: Vec<Key> = bindings.axis.get("BTN_DPAD_UP")
            .unwrap_or(&Vec::new()).clone();
        pad_y.extend(bindings.axis.get("BTN_DPAD_DOWN")
            .unwrap_or(&Vec::new()));
        bindings.axis.insert("BTN_DPAD_X".to_string(), pad_x);
        bindings.axis.insert("BTN_DPAD_Y".to_string(), pad_y);

        let mut lstick_x: Vec<Key> = bindings.axis.get("LSTICK_LEFT")
            .unwrap_or(&Vec::new()).clone();
        lstick_x.extend(bindings.axis.get("LSTICK_RIGHT")
            .unwrap_or(&Vec::new()));
        let mut lstick_y: Vec<Key> = bindings.axis.get("LSTICK_UP")
            .unwrap_or(&Vec::new()).clone();
        lstick_y.extend(bindings.axis.get("LSTICK_DOWN")
            .unwrap_or(&Vec::new()));
        bindings.axis.insert("LSTICK_X".to_string(), lstick_x);
        bindings.axis.insert("LSTICK_Y".to_string(), lstick_y);

        let mut rstick_x: Vec<Key> = bindings.axis.get("RSTICK_LEFT")
            .unwrap_or(&Vec::new()).clone();
        rstick_x.extend(bindings.axis.get("RSTICK_RIGHT")
            .unwrap_or(&Vec::new()));
        let mut rstick_y: Vec<Key> = bindings.axis.get("RSTICK_UP")
            .unwrap_or(&Vec::new()).clone();
        rstick_y.extend(bindings.axis.get("RSTICK_DOWN")
            .unwrap_or(&Vec::new()));
        bindings.axis.insert("RSTICK_X".to_string(), rstick_x);
        bindings.axis.insert("RSTICK_Y".to_string(), rstick_y);
        bindings
}


