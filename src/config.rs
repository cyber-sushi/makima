use std::collections::{HashMap, BTreeMap};
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
    pub keys: HashMap<String, HashMap<Key, Vec<Key>>>,
    pub axis: HashMap<String, HashMap<String, Vec<Key>>>,
    pub keys_sh: HashMap<String, HashMap<Key, Vec<String>>>,
    pub axis_sh: HashMap<String, HashMap<String, Vec<String>>>,
}

#[derive(Default, Debug, Clone)]
pub struct Modifiers {
    pub keys: HashMap<BTreeMap<Key, i32>, HashMap<Key, Vec<Key>>>,
    pub axis: HashMap<BTreeMap<Key, i32>, HashMap<String, Vec<Key>>>,
    pub keys_sh: HashMap<BTreeMap<Key, i32>, HashMap<Key, Vec<String>>>,
    pub axis_sh: HashMap<BTreeMap<Key, i32>, HashMap<String, Vec<String>>>,
}


#[derive(serde::Deserialize, Debug, Clone)]
pub struct RawConfig {
    #[serde(default)]
    pub remap: HashMap<String, Vec<Key>>,
    #[serde(default)]
    pub commands: HashMap<String, Vec<String>>,
    pub settings: HashMap<String, String>,
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
    pub modifiers: Modifiers,
    pub settings: HashMap<String, String>,
}

impl Config {
    pub fn new_from_file(file: &str, file_name: String) -> Self {
        let raw_config = RawConfig::new_from_file(file);
        let (bindings, combinations, settings) = parse_raw_config(raw_config);
        let bindings: Bindings = merge_axis_bindings(bindings);
        let modifiers: Modifiers = parse_modifiers(combinations);

        Self {
            name: file_name,
            bindings,
            modifiers,
            settings,
        }
    }
}

fn parse_raw_config(raw_config: RawConfig) -> (Bindings, Combinations, HashMap<String, String>) {
    let remap: HashMap<String, Vec<Key>> = raw_config.remap;
    let commands: HashMap<String, Vec<String>> = raw_config.commands;
    let settings: HashMap<String, String> = raw_config.settings;
    let mut bindings: Bindings = Default::default();
    let mut combinations: Combinations = Default::default();

    let abs = [
        "DPAD_UP",
        "DPAD_DOWN",
        "DPAD_LEFT",
        "DPAD_RIGHT",
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
            if abs.contains(&key) {
                if !combinations.axis.contains_key(&mods.to_string()) {
                    combinations.axis.insert(mods.to_string(), HashMap::from([(key.to_string(), output)]));
                } else {
                    combinations.axis.get_mut(mods).unwrap().insert(key.to_string(), output);
                }
            } else {
                if !combinations.keys.contains_key(&mods.to_string()) {
                    combinations.keys.insert(mods.to_string(), HashMap::from([(Key::from_str(key).expect("Invalid KEY value."), output)]));
                } else {
                    combinations.keys.get_mut(mods).unwrap().insert(Key::from_str(key).expect("Invalid KEY value."), output);
                }
            }
        } else {
            if abs.contains(&input.as_str()) {
                bindings.axis.insert(input, output);
            } else {
                bindings.keys.insert(Key::from_str(input.as_str()).expect("Invalid KEY value."), output);
            }
        }
    }

    for (input, output) in commands.clone().into_iter() {
        if input.contains("-") {
            let (mods, key) = input.rsplit_once("-").unwrap();
            if abs.contains(&key) {
                if !combinations.axis_sh.contains_key(&mods.to_string()) {
                    combinations.axis_sh.insert(mods.to_string(), HashMap::from([(key.to_string(), output)]));
                } else {
                    combinations.axis_sh.get_mut(mods).unwrap().insert(key.to_string(), output);
                }
            } else {
                if !combinations.keys_sh.contains_key(&mods.to_string()) {
                    combinations.keys_sh.insert(mods.to_string(), HashMap::from([(Key::from_str(key).expect("Invalid KEY value."), output)]));
                } else {
                    combinations.keys_sh.get_mut(mods).unwrap().insert(Key::from_str(key).expect("Invalid KEY value."), output);
                }
            }
        } else {
            if abs.contains(&input.as_str()) {
                bindings.axis_sh.insert(input, output);
            } else {
                bindings.keys_sh.insert(Key::from_str(input.as_str()).expect("Invalid KEY value."), output);
            }
        }
    }
    (bindings, combinations, settings)
}

fn parse_modifiers(combinations: Combinations) -> Modifiers {
    let empty_modmap = BTreeMap::from ([
        (Key::KEY_LEFTSHIFT, 0),
        (Key::KEY_LEFTCTRL, 0),
        (Key::KEY_LEFTALT, 0),
        (Key::KEY_RIGHTSHIFT, 0),
        (Key::KEY_RIGHTCTRL, 0),
        (Key::KEY_RIGHTALT, 0),
        (Key::KEY_LEFTMETA, 0)
    ]);
    let mut modifiers: Modifiers = Default::default();
    for (mods, key) in combinations.keys.iter() {
        let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
        let mut modmap = empty_modmap.clone();
        for modifier in mods_vector {
            modmap.insert(Key::from_str(&modifier).unwrap(), 1);
        }
        modifiers.keys.insert(modmap, key.clone());
    }
    for (mods, key) in combinations.axis.iter() {
        let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
        let mut modmap = empty_modmap.clone();
        for modifier in mods_vector {
            modmap.insert(Key::from_str(&modifier).unwrap(), 1);
        }
        modifiers.axis.insert(modmap, key.clone());
    }
    for (mods, key) in combinations.keys_sh.iter() {
        let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
        let mut modmap = empty_modmap.clone();
        for modifier in mods_vector {
            modmap.insert(Key::from_str(&modifier).unwrap(), 1);
        }
        modifiers.keys_sh.insert(modmap, key.clone());
    }
    for (mods, key) in combinations.axis_sh.iter() {
        let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
        let mut modmap = empty_modmap.clone();
        for modifier in mods_vector {
            modmap.insert(Key::from_str(&modifier).unwrap(), 1);
        }
        modifiers.axis_sh.insert(modmap, key.clone());
    }
    modifiers
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
        bindings.axis.insert("BTN_DPAD_y".to_string(), pad_y);

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
