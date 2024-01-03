use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;
use evdev::Key;
use serde;


#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct Bindings {
    #[serde(default)]
    pub keys: HashMap<Key, Vec<Key>>,
    #[serde(default)]
    pub axis: HashMap<String, Vec<Key>>,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct Combinations {
    #[serde(default)]
    pub keys: HashMap<String, HashMap<Key, Vec<Key>>>,
    #[serde(default)]
    pub axis: HashMap<String, HashMap<String, Vec<Key>>>,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct Modifiers {
    pub keys: HashMap<BTreeMap<Key, i32>, HashMap<Key, Vec<Key>>>,
    pub axis: HashMap<BTreeMap<Key, i32>, HashMap<String, Vec<Key>>>,
}

impl Modifiers {
    pub fn new() -> Self {
        let keys: HashMap<BTreeMap<Key, i32>, HashMap<Key, Vec<Key>>> = HashMap::new();
        let axis: HashMap<BTreeMap<Key, i32>, HashMap<String, Vec<Key>>> = HashMap::new();
        Self {
            keys: keys,
            axis: axis
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub name: String,
    #[serde(default)]
    pub bindings: Bindings,
    #[serde(default)]
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
        let mut bindings: Bindings = config.bindings;
        let combinations: Combinations = config.combinations;
        let settings: HashMap<String, String> = config.settings;
        
        let empty_modmap = BTreeMap::from ([
                                (Key::KEY_LEFTSHIFT, 0),
                                (Key::KEY_LEFTCTRL, 0),
                                (Key::KEY_LEFTALT, 0),
                                (Key::KEY_RIGHTSHIFT, 0),
                                (Key::KEY_RIGHTCTRL, 0),
                                (Key::KEY_RIGHTALT, 0),
                                (Key::KEY_LEFTMETA, 0)
        ]);
        let mut modifiers = Modifiers::new();
        for (mods, map) in combinations.keys.iter() {
            let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
            let mut modmap = empty_modmap.clone();
            for modifier in mods_vector {
                modmap.insert(Key::from_str(&modifier).unwrap(), 1);
            }
            modifiers.keys.insert(modmap, map.clone());
        }
        
        for (mods, map) in combinations.axis.iter() {
            let mods_vector = mods.split("-").map(str::to_string).collect::<Vec<String>>();
            let mut modmap = empty_modmap.clone();
            for modifier in mods_vector {
                modmap.insert(Key::from_str(&modifier).unwrap(), 1);
            }
            modifiers.axis.insert(modmap, map.clone());
        }

        let mut pad_horizontal: Vec<Key> = bindings.axis.get("BTN_DPAD_LEFT")
            .unwrap_or(&Vec::new()).clone();
        pad_horizontal.extend(bindings.axis.get("BTN_DPAD_RIGHT")
            .unwrap_or(&Vec::new()));
        let mut pad_vertical: Vec<Key> = bindings.axis.get("BTN_DPAD_UP")
            .unwrap_or(&Vec::new()).clone();
        pad_vertical.extend(bindings.axis.get("BTN_DPAD_DOWN")
            .unwrap_or(&Vec::new()));
        bindings.axis.insert("NONE_X".to_string(), pad_horizontal);
        bindings.axis.insert("NONE_Y".to_string(), pad_vertical);

        Self {
            name: file_name,
            bindings: bindings,
            combinations: combinations,
            modifiers: modifiers,
            settings: settings,
        }
    }
}
