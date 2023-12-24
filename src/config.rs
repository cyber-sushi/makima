use std::collections::HashMap;
use evdev::Key;
use serde;


#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub name: String,
    #[serde(default)]
    pub keys: HashMap<Key, Vec<Key>>,
    pub settings: HashMap<String, String>,
    #[serde(skip)]
    pub abs: HashMap<String, Vec<Key>>,
    #[serde(default)]
    pub rel: HashMap<String, Vec<Key>>,
}

impl Config {
    pub fn new_from_file(file: &str, file_name: String) -> Self {
        println!("Parsing config file at {:?}", file);
        let file_content: String = std::fs::read_to_string(file).unwrap();
        let config: Config = toml::from_str(&file_content)
            .expect("Couldn't parse config file.");
        let keys: HashMap<Key, Vec<Key>> = config.keys;
        let rel: HashMap<String, Vec<Key>> = config.rel;
        let mut abs: HashMap<String, Vec<Key>> = HashMap::new();
        let mut pad_horizontal: Vec<Key> = keys.get(&Key::BTN_DPAD_LEFT)
            .unwrap_or(&Vec::new()).clone();
        pad_horizontal.extend(keys.get(&Key::BTN_DPAD_RIGHT)
            .unwrap_or(&Vec::new()));
        let mut pad_vertical: Vec<Key> = keys.get(&Key::BTN_DPAD_UP)
            .unwrap_or(&Vec::new()).clone();
        pad_vertical.extend(keys.get(&Key::BTN_DPAD_DOWN)
            .unwrap_or(&Vec::new()));
        abs.insert("NONE_X".to_string(), pad_horizontal);
        abs.insert("NONE_Y".to_string(), pad_vertical);
        let settings: HashMap<String, String> = config.settings;
        Self {
            name: file_name,
            keys: keys,
            settings: settings,
            abs: abs,
            rel: rel
        }
    }
}

