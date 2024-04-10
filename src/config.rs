use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;
use evdev::Key;
use serde;


#[derive(Default, Debug, Clone)]
pub struct Bindings {
    pub keys: HashMap<Key, Vec<Key>>,
    pub axis: HashMap<String, Vec<Key>>,
}

#[derive(Default, Debug, Clone)]
pub struct Combinations {
    pub keys: HashMap<String, HashMap<Key, Vec<Key>>>,
    pub axis: HashMap<String, HashMap<String, Vec<Key>>>,
}

#[derive(Default, Debug, Clone)]
pub struct Modifiers {
    pub keys: HashMap<BTreeMap<Key, i32>, HashMap<Key, Vec<Key>>>,
    pub axis: HashMap<BTreeMap<Key, i32>, HashMap<String, Vec<Key>>>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub name: String,
    #[serde(default)]
    pub remap: HashMap<String, Vec<Key>>,
    #[serde(skip)]
    pub bindings: Bindings,
    #[serde(skip)]
    pub combinations: Combinations,
    #[serde(skip)]
    pub modifiers: Modifiers,
    pub settings: HashMap<String, String>,
}

impl Config {
    pub fn new_from_file(file: &str, file_name: String) -> Self {
        println!("Parsing config file:\n{:?}\n", file);
        let file_content: String = std::fs::read_to_string(file).unwrap();
        let config: Config = toml::from_str(&file_content)
            .expect("Couldn't parse config file.");
        let settings: HashMap<String, String> = config.settings;
        let remap: HashMap<String, Vec<Key>> = config.remap;
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
            "SCROLLWHEEL_UP",
            "SCROLLWHEEL_DOWN",
            "BTN_TL2",
            "BTN_TR2",
        ];
        for (input, output) in remap.clone().into_iter() {
            if input.contains(".") {
                let (mods, key) = input.split_once(".").unwrap();
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
            let mods_vector = mods.split("+").map(str::to_string).collect::<Vec<String>>();
            let mut modmap = empty_modmap.clone();
            for modifier in mods_vector {
                modmap.insert(Key::from_str(&modifier).unwrap(), 1);
            }
            modifiers.keys.insert(modmap, key.clone());
        }
        
        for (mods, key) in combinations.axis.iter() {
            let mods_vector = mods.split("+").map(str::to_string).collect::<Vec<String>>();
            let mut modmap = empty_modmap.clone();
            for modifier in mods_vector {
                modmap.insert(Key::from_str(&modifier).unwrap(), 1);
            }
            modifiers.axis.insert(modmap, key.clone());
        }

        let mut pad_x: Vec<Key> = bindings.axis.get("BTN_DPAD_LEFT")
            .unwrap_or(&Vec::new()).clone();
        pad_x.extend(bindings.axis.get("BTN_DPAD_RIGHT")
            .unwrap_or(&Vec::new()));
        let mut pad_y: Vec<Key> = bindings.axis.get("BTN_DPAD_UP")
            .unwrap_or(&Vec::new()).clone();
        pad_y.extend(bindings.axis.get("BTN_DPAD_DOWN")
            .unwrap_or(&Vec::new()));
        bindings.axis.insert("NONE_X".to_string(), pad_x);
        bindings.axis.insert("NONE_Y".to_string(), pad_y);

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

        Self {
            name: file_name,
            remap,
            bindings,
            combinations,
            modifiers,
            settings,
        }
    }
}
