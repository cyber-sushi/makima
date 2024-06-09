use crate::active_client::*;
use crate::config::{parse_modifiers, Associations, Axis, Event};
use crate::udev_monitor::Environment;
use crate::virtual_devices::VirtualDevices;
use crate::Config;
use evdev::{AbsoluteAxisType, EventStream, EventType, InputEvent, Key, RelativeAxisType};
use fork::{fork, setsid, Fork};
use std::{
    future::Future,
    option::Option,
    pin::Pin,
    process::{Command, Stdio},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

struct Stick {
    function: String,
    sensitivity: u64,
    deadzone: i32,
    activation_modifiers: Vec<Event>,
}

struct Settings {
    lstick: Stick,
    rstick: Stick,
    invert_cursor_axis: bool,
    invert_scroll_axis: bool,
    axis_16_bit: bool,
    chain_only: bool,
    layout_switcher: Key,
    notify_layout_switch: bool,
}

pub struct EventReader {
    config: Vec<Config>,
    stream: Arc<Mutex<EventStream>>,
    virt_dev: Arc<Mutex<VirtualDevices>>,
    lstick_position: Arc<Mutex<Vec<i32>>>,
    rstick_position: Arc<Mutex<Vec<i32>>>,
    modifiers: Arc<Mutex<Vec<Event>>>,
    modifier_was_activated: Arc<Mutex<bool>>,
    device_is_connected: Arc<Mutex<bool>>,
    active_layout: Arc<Mutex<u16>>,
    current_config: Arc<Mutex<Config>>,
    environment: Environment,
    settings: Settings,
}

impl EventReader {
    pub fn new(
        config: Vec<Config>,
        stream: Arc<Mutex<EventStream>>,
        modifiers: Arc<Mutex<Vec<Event>>>,
        modifier_was_activated: Arc<Mutex<bool>>,
        environment: Environment,
    ) -> Self {
        let mut position_vector: Vec<i32> = Vec::new();
        for i in [0, 0] {
            position_vector.push(i)
        }
        let lstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let rstick_position = Arc::new(Mutex::new(position_vector.clone()));
        let device_is_connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        let active_layout: Arc<Mutex<u16>> = Arc::new(Mutex::new(0));
        let current_config: Arc<Mutex<Config>> = Arc::new(Mutex::new(
            config
                .iter()
                .find(|&x| x.associations == Associations::default())
                .unwrap()
                .clone(),
        ));
        let virt_dev = Arc::new(Mutex::new(VirtualDevices::new()));
        let lstick_function = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("LSTICK")
            .unwrap_or(&"cursor".to_string())
            .to_string();
        let lstick_sensitivity: u64 = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("LSTICK_SENSITIVITY")
            .unwrap_or(&"0".to_string())
            .parse::<u64>()
            .expect("Invalid value for LSTICK_SENSITIVITY, please use an integer value >= 0");
        let lstick_deadzone: i32 = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("LSTICK_DEADZONE")
            .unwrap_or(&"5".to_string())
            .parse::<i32>()
            .expect("Invalid value for LSTICK_DEADZONE, please use an integer between 0 and 128.");
        let lstick_activation_modifiers: Vec<Event> = parse_modifiers(
            &config
                .iter()
                .find(|&x| x.associations == Associations::default())
                .unwrap()
                .settings,
            "LSTICK_ACTIVATION_MODIFIERS",
        );
        let lstick = Stick {
            function: lstick_function,
            sensitivity: lstick_sensitivity,
            deadzone: lstick_deadzone,
            activation_modifiers: lstick_activation_modifiers,
        };

        let rstick_function: String = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("RSTICK")
            .unwrap_or(&"scroll".to_string())
            .to_string();
        let rstick_sensitivity: u64 = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("RSTICK_SENSITIVITY")
            .unwrap_or(&"0".to_string())
            .parse::<u64>()
            .expect("Invalid value for RSTICK_SENSITIVITY, please use an integer value >= 0");
        let rstick_deadzone: i32 = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("RSTICK_DEADZONE")
            .unwrap_or(&"5".to_string())
            .parse::<i32>()
            .expect("Invalid value for RSTICK_DEADZONE, please use an integer between 0 and 128.");
        let rstick_activation_modifiers: Vec<Event> = parse_modifiers(
            &config
                .iter()
                .find(|&x| x.associations == Associations::default())
                .unwrap()
                .settings,
            "RSTICK_ACTIVATION_MODIFIERS",
        );
        let rstick = Stick {
            function: rstick_function,
            sensitivity: rstick_sensitivity,
            deadzone: rstick_deadzone,
            activation_modifiers: rstick_activation_modifiers,
        };

        let axis_16_bit: bool = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("16_BIT_AXIS")
            .unwrap_or(&"false".to_string())
            .parse()
            .expect("16_BIT_AXIS can only be true or false.");

        let chain_only: bool = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("CHAIN_ONLY")
            .unwrap_or(&"true".to_string())
            .parse()
            .expect("CHAIN_ONLY can only be true or false.");

        let invert_cursor_axis: bool = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("INVERT_CURSOR_AXIS")
            .unwrap_or(&"false".to_string())
            .parse()
            .expect("INVERT_CURSOR_AXIS can only be true or false.");

        let invert_scroll_axis: bool = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("INVERT_SCROLL_AXIS")
            .unwrap_or(&"false".to_string())
            .parse()
            .expect("INVERT_SCROLL_AXIS can only be true or false.");

        let layout_switcher: Key = Key::from_str(
            config
                .iter()
                .find(|&x| x.associations == Associations::default())
                .unwrap()
                .settings
                .get("LAYOUT_SWITCHER")
                .unwrap_or(&"BTN_0".to_string()),
        )
        .expect("LAYOUT_SWITCHER is not a valid Key.");

        let notify_layout_switch: bool = config
            .iter()
            .find(|&x| x.associations == Associations::default())
            .unwrap()
            .settings
            .get("NOTIFY_LAYOUT_SWITCH")
            .unwrap_or(&"false".to_string())
            .parse()
            .expect("NOTIFY_LAYOUT_SWITCH can only be true or false.");

        let settings = Settings {
            lstick,
            rstick,
            invert_cursor_axis,
            invert_scroll_axis,
            axis_16_bit,
            chain_only,
            layout_switcher,
            notify_layout_switch,
        };
        Self {
            config,
            stream,
            virt_dev,
            lstick_position,
            rstick_position,
            modifiers,
            modifier_was_activated,
            device_is_connected,
            active_layout,
            current_config,
            environment,
            settings,
        }
    }

    pub async fn start(&self) {
        println!(
            "{:?} detected, reading events.\n",
            self.config
                .iter()
                .find(|&x| x.associations == Associations::default())
                .unwrap()
                .name
        );
        tokio::join!(self.event_loop(), self.cursor_loop(), self.scroll_loop(),);
    }

    pub async fn event_loop(&self) {
        let (
            mut dpad_values,
            mut lstick_values,
            mut rstick_values,
            mut triggers_values,
            mut abs_wheel_position,
        ) = ((0, 0), (0, 0), (0, 0), (0, 0), 0);
        let switcher: Key = self.settings.layout_switcher;
        let mut stream = self.stream.lock().await;
        let mut max_abs_wheel = 0;
        if let Ok(abs_state) = stream.device().get_abs_state() {
            for state in abs_state {
                if state.maximum > max_abs_wheel {
                    max_abs_wheel = state.maximum;
                }
            }
        }
        while let Some(Ok(event)) = stream.next().await {
            match (
                event.event_type(),
                RelativeAxisType(event.code()),
                AbsoluteAxisType(event.code()),
            ) {
                (EventType::KEY, _, _) => match Key(event.code()) {
                    Key::BTN_TL2 | Key::BTN_TR2 => {}
                    key if key == switcher && event.value() == 1 => {
                        self.change_active_layout().await
                    }
                    _ => {
                        self.convert_event(
                            event,
                            Event::Key(Key(event.code())),
                            event.value(),
                            false,
                        )
                        .await
                    }
                },
                (
                    EventType::RELATIVE,
                    RelativeAxisType::REL_WHEEL | RelativeAxisType::REL_WHEEL_HI_RES,
                    _,
                ) => match event.value() {
                    -1 => {
                        self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_DOWN), 1, true)
                            .await;
                    }
                    1 => {
                        self.convert_event(event, Event::Axis(Axis::SCROLL_WHEEL_UP), 1, true)
                            .await;
                    }
                    _ => {}
                },
                (_, _, AbsoluteAxisType::ABS_HAT0X) => {
                    match event.value() {
                        -1 => {
                            self.convert_event(event, Event::Axis(Axis::BTN_DPAD_LEFT), 1, false)
                                .await;
                            dpad_values.0 = -1;
                        }
                        1 => {
                            self.convert_event(event, Event::Axis(Axis::BTN_DPAD_RIGHT), 1, false)
                                .await;
                            dpad_values.0 = 1;
                        }
                        0 => {
                            match dpad_values.0 {
                                -1 => {
                                    self.convert_event(
                                        event,
                                        Event::Axis(Axis::BTN_DPAD_LEFT),
                                        0,
                                        false,
                                    )
                                    .await
                                }
                                1 => {
                                    self.convert_event(
                                        event,
                                        Event::Axis(Axis::BTN_DPAD_RIGHT),
                                        0,
                                        false,
                                    )
                                    .await
                                }
                                _ => {}
                            }
                            dpad_values.0 = 0;
                        }
                        _ => {}
                    };
                }
                (_, _, AbsoluteAxisType::ABS_HAT0Y) => {
                    match event.value() {
                        -1 => {
                            self.convert_event(event, Event::Axis(Axis::BTN_DPAD_UP), 1, false)
                                .await;
                            dpad_values.1 = -1;
                        }
                        1 => {
                            self.convert_event(event, Event::Axis(Axis::BTN_DPAD_DOWN), 1, false)
                                .await;
                            dpad_values.1 = 1;
                        }
                        0 => {
                            match dpad_values.1 {
                                -1 => {
                                    self.convert_event(
                                        event,
                                        Event::Axis(Axis::BTN_DPAD_UP),
                                        0,
                                        false,
                                    )
                                    .await
                                }
                                1 => {
                                    self.convert_event(
                                        event,
                                        Event::Axis(Axis::BTN_DPAD_DOWN),
                                        0,
                                        false,
                                    )
                                    .await
                                }
                                _ => {}
                            }
                            dpad_values.1 = 0;
                        }
                        _ => {}
                    };
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y) => {
                    match self.settings.lstick.function.as_str() {
                        "cursor" | "scroll" => {
                            let axis_value = self
                                .get_axis_value(&event, &self.settings.lstick.deadzone)
                                .await;
                            let mut lstick_position = self.lstick_position.lock().await;
                            lstick_position[event.code() as usize] = axis_value;
                        }
                        "bind" => {
                            let axis_value = self
                                .get_axis_value(&event, &self.settings.lstick.deadzone)
                                .await;
                            let clamped_value = if axis_value < 0 {
                                -1
                            } else if axis_value > 0 {
                                1
                            } else {
                                0
                            };
                            match AbsoluteAxisType(event.code()) {
                                AbsoluteAxisType::ABS_Y => match clamped_value {
                                    -1 if lstick_values.1 != -1 => {
                                        self.convert_event(
                                            event,
                                            Event::Axis(Axis::LSTICK_UP),
                                            1,
                                            false,
                                        )
                                        .await;
                                        lstick_values.1 = -1
                                    }
                                    1 if lstick_values.1 != 1 => {
                                        self.convert_event(
                                            event,
                                            Event::Axis(Axis::LSTICK_DOWN),
                                            1,
                                            false,
                                        )
                                        .await;
                                        lstick_values.1 = 1
                                    }
                                    0 => {
                                        if lstick_values.1 != 0 {
                                            match lstick_values.1 {
                                                -1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::LSTICK_UP),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::LSTICK_DOWN),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                _ => {}
                                            }
                                            lstick_values.1 = 0;
                                        }
                                    }
                                    _ => {}
                                },
                                AbsoluteAxisType::ABS_X => match clamped_value {
                                    -1 if lstick_values.0 != -1 => {
                                        self.convert_event(
                                            event,
                                            Event::Axis(Axis::LSTICK_LEFT),
                                            1,
                                            false,
                                        )
                                        .await;
                                        lstick_values.0 = -1
                                    }
                                    1 => {
                                        if lstick_values.0 != 1 {
                                            self.convert_event(
                                                event,
                                                Event::Axis(Axis::LSTICK_RIGHT),
                                                1,
                                                false,
                                            )
                                            .await;
                                            lstick_values.0 = 1
                                        }
                                    }
                                    0 => {
                                        if lstick_values.0 != 0 {
                                            match lstick_values.0 {
                                                -1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::LSTICK_LEFT),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::LSTICK_RIGHT),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                _ => {}
                                            }
                                            lstick_values.0 = 0;
                                        }
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RX | AbsoluteAxisType::ABS_RY) => {
                    match self.settings.rstick.function.as_str() {
                        "cursor" | "scroll" => {
                            let axis_value = self
                                .get_axis_value(&event, &self.settings.rstick.deadzone)
                                .await;
                            let mut rstick_position = self.rstick_position.lock().await;
                            rstick_position[event.code() as usize - 3] = axis_value;
                        }
                        "bind" => {
                            let axis_value = self
                                .get_axis_value(&event, &self.settings.rstick.deadzone)
                                .await;
                            let clamped_value = if axis_value < 0 {
                                -1
                            } else if axis_value > 0 {
                                1
                            } else {
                                0
                            };
                            match AbsoluteAxisType(event.code()) {
                                AbsoluteAxisType::ABS_RY => match clamped_value {
                                    -1 => {
                                        if rstick_values.1 != -1 {
                                            self.convert_event(
                                                event,
                                                Event::Axis(Axis::RSTICK_UP),
                                                1,
                                                false,
                                            )
                                            .await;
                                            rstick_values.1 = -1
                                        }
                                    }
                                    1 => {
                                        if rstick_values.1 != 1 {
                                            self.convert_event(
                                                event,
                                                Event::Axis(Axis::RSTICK_DOWN),
                                                1,
                                                false,
                                            )
                                            .await;
                                            rstick_values.1 = 1
                                        }
                                    }
                                    0 => {
                                        if rstick_values.1 != 0 {
                                            match rstick_values.1 {
                                                -1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::RSTICK_UP),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::RSTICK_DOWN),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                _ => {}
                                            }
                                            rstick_values.1 = 0;
                                        }
                                    }
                                    _ => {}
                                },
                                AbsoluteAxisType::ABS_RX => match clamped_value {
                                    -1 if rstick_values.0 != -1 => {
                                        self.convert_event(
                                            event,
                                            Event::Axis(Axis::RSTICK_LEFT),
                                            1,
                                            false,
                                        )
                                        .await;
                                        rstick_values.0 = -1
                                    }
                                    1 => {
                                        if rstick_values.0 != 1 {
                                            self.convert_event(
                                                event,
                                                Event::Axis(Axis::RSTICK_RIGHT),
                                                1,
                                                false,
                                            )
                                            .await;
                                            rstick_values.0 = 1
                                        }
                                    }
                                    0 => {
                                        if rstick_values.0 != 0 {
                                            match rstick_values.0 {
                                                -1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::RSTICK_LEFT),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                1 => {
                                                    self.convert_event(
                                                        event,
                                                        Event::Axis(Axis::RSTICK_RIGHT),
                                                        0,
                                                        false,
                                                    )
                                                    .await
                                                }
                                                _ => {}
                                            }
                                            rstick_values.0 = 0;
                                        }
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_Z) => {
                    match (event.value(), triggers_values.0) {
                        (0, 1) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TL2), 0, false)
                                .await;
                            triggers_values.0 = 0;
                        }
                        (_, 0) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TL2), 1, false)
                                .await;
                            triggers_values.0 = 1;
                        }
                        _ => {}
                    }
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_RZ) => {
                    match (event.value(), triggers_values.1) {
                        (0, 1) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TR2), 0, false)
                                .await;
                            triggers_values.1 = 0;
                        }
                        (_, 0) => {
                            self.convert_event(event, Event::Axis(Axis::BTN_TR2), 1, false)
                                .await;
                            triggers_values.1 = 1;
                        }
                        _ => {}
                    }
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_WHEEL) => {
                    let value = event.value();
                    if value != 0 && abs_wheel_position != 0 {
                        let gap = value - abs_wheel_position;
                        if gap < -max_abs_wheel / 2 {
                            self.convert_event(event, Event::Axis(Axis::ABS_WHEEL_CW), 1, true)
                                .await;
                        } else if gap > max_abs_wheel / 2 {
                            self.convert_event(event, Event::Axis(Axis::ABS_WHEEL_CCW), 1, true)
                                .await;
                        } else if value > abs_wheel_position {
                            self.convert_event(event, Event::Axis(Axis::ABS_WHEEL_CW), 1, true)
                                .await;
                        } else if value < abs_wheel_position {
                            self.convert_event(event, Event::Axis(Axis::ABS_WHEEL_CCW), 1, true)
                                .await;
                        }
                    }
                    abs_wheel_position = value;
                }
                (EventType::ABSOLUTE, _, AbsoluteAxisType::ABS_MISC) => {
                    if event.value() == 0 {
                        abs_wheel_position = 0
                    };
                }
                _ => self.emit_default_event(event).await,
            }
        }
        let mut device_is_connected = self.device_is_connected.lock().await;
        *device_is_connected = false;

        println!(
            "Disconnected device \"{}\".\n",
            self.current_config.lock().await.name
        );
    }

    async fn convert_event(
        &self,
        default_event: InputEvent,
        event: Event,
        value: i32,
        send_zero: bool,
    ) {
        if value == 1 {
            self.update_config().await;
        };
        let config = self.current_config.lock().await;
        let modifiers = self.modifiers.lock().await.clone();
        if let Some(map) = config.bindings.remap.get(&event) {
            if let Some(event_list) = map.get(&modifiers) {
                self.emit_event(
                    event_list,
                    value,
                    &modifiers,
                    &config,
                    modifiers.is_empty(),
                    !modifiers.is_empty(),
                )
                .await;
                if send_zero {
                    let modifiers = self.modifiers.lock().await.clone();
                    self.emit_event(
                        event_list,
                        0,
                        &modifiers,
                        &config,
                        modifiers.is_empty(),
                        !modifiers.is_empty(),
                    )
                    .await;
                }
                return;
            }
            if let Some(event_list) = map.get(&vec![Event::Hold]) {
                if !modifiers.is_empty() || self.settings.chain_only == false {
                    self.emit_event(event_list, value, &modifiers, &config, false, false)
                        .await;
                    return;
                }
            }
            if let Some(map) = config.bindings.commands.get(&event) {
                if let Some(command_list) = map.get(&modifiers) {
                    if value == 1 {
                        self.spawn_subprocess(command_list).await
                    };
                    return;
                }
            }
            if let Some(event_list) = map.get(&Vec::new()) {
                self.emit_event(event_list, value, &modifiers, &config, true, false)
                    .await;
                if send_zero {
                    let modifiers = self.modifiers.lock().await.clone();
                    self.emit_event(event_list, 0, &modifiers, &config, true, false)
                        .await;
                }
                return;
            }
        }
        if let Some(map) = config.bindings.commands.get(&event) {
            if let Some(command_list) = map.get(&modifiers) {
                if value == 1 {
                    self.spawn_subprocess(command_list).await
                };
                return;
            }
        }
        self.emit_nonmapped_event(default_event, event, value, &modifiers, &config)
            .await;
    }

    async fn emit_event(
        &self,
        event_list: &Vec<Key>,
        value: i32,
        modifiers: &Vec<Event>,
        config: &Config,
        release_keys: bool,
        ignore_modifiers: bool,
    ) {
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        if release_keys {
            let released_keys: Vec<Key> = self.released_keys(&modifiers, &config).await;
            for key in released_keys {
                self.toggle_modifiers(Event::Key(key), 0, &config).await;
                let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
            }
        } else if ignore_modifiers {
            for key in modifiers.iter() {
                if let Event::Key(key) = key {
                    let virtual_event: InputEvent =
                        InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                }
            }
        }
        for key in event_list {
            if release_keys {
                self.toggle_modifiers(Event::Key(*key), value, &config)
                    .await;
            }
            if config.mapped_modifiers.custom.contains(&Event::Key(*key)) {
                if value == 0 && !*modifier_was_activated {
                    let virtual_event: InputEvent =
                        InputEvent::new_now(EventType::KEY, key.code(), 1);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    let virtual_event: InputEvent =
                        InputEvent::new_now(EventType::KEY, key.code(), 0);
                    virt_dev.keys.emit(&[virtual_event]).unwrap();
                    *modifier_was_activated = true;
                } else if value == 1 {
                    *modifier_was_activated = false;
                }
            } else {
                let virtual_event: InputEvent =
                    InputEvent::new_now(EventType::KEY, key.code(), value);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                *modifier_was_activated = true;
            }
        }
    }

    async fn emit_nonmapped_event(
        &self,
        default_event: InputEvent,
        event: Event,
        value: i32,
        modifiers: &Vec<Event>,
        config: &Config,
    ) {
        let mut virt_dev = self.virt_dev.lock().await;
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        let released_keys: Vec<Key> = self.released_keys(&modifiers, &config).await;
        for key in released_keys {
            self.toggle_modifiers(Event::Key(key), 0, &config).await;
            let virtual_event: InputEvent = InputEvent::new_now(EventType::KEY, key.code(), 0);
            virt_dev.keys.emit(&[virtual_event]).unwrap()
        }
        self.toggle_modifiers(event, value, &config).await;
        if config.mapped_modifiers.custom.contains(&event) {
            if value == 0 && !*modifier_was_activated {
                let virtual_event: InputEvent =
                    InputEvent::new_now(default_event.event_type(), default_event.code(), 1);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                let virtual_event: InputEvent =
                    InputEvent::new_now(default_event.event_type(), default_event.code(), 0);
                virt_dev.keys.emit(&[virtual_event]).unwrap();
                *modifier_was_activated = true;
            } else if value == 1 {
                *modifier_was_activated = false;
            }
        } else {
            *modifier_was_activated = true;
            match default_event.event_type() {
                EventType::KEY => {
                    virt_dev.keys.emit(&[default_event]).unwrap();
                }
                EventType::RELATIVE => {
                    virt_dev.axis.emit(&[default_event]).unwrap();
                }
                _ => {}
            }
        }
    }

    async fn emit_default_event(&self, event: InputEvent) {
        match event.event_type() {
            EventType::KEY => {
                let mut virt_dev = self.virt_dev.lock().await;
                virt_dev.keys.emit(&[event]).unwrap();
            }
            EventType::RELATIVE => {
                let mut virt_dev = self.virt_dev.lock().await;
                virt_dev.axis.emit(&[event]).unwrap();
            }
            _ => {}
        }
    }

    async fn spawn_subprocess(&self, command_list: &Vec<String>) {
        let mut modifier_was_activated = self.modifier_was_activated.lock().await;
        *modifier_was_activated = true;
        let (user, running_as_root) = if let Ok(sudo_user) = &self.environment.sudo_user {
            (Option::Some(sudo_user), true)
        } else if let Ok(user) = &self.environment.user {
            (Option::Some(user), false)
        } else {
            (Option::None, false)
        };
        if let Some(user) = user {
            for command in command_list {
                if running_as_root {
                    match fork() {
                        Ok(Fork::Child) => match fork() {
                            Ok(Fork::Child) => {
                                setsid().unwrap();
                                Command::new("runuser")
                                    .args([user, "-c", command])
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .spawn()
                                    .unwrap();
                                std::process::exit(0);
                            }
                            Ok(Fork::Parent(_)) => std::process::exit(0),
                            Err(_) => std::process::exit(1),
                        },
                        Ok(Fork::Parent(_)) => (),
                        Err(_) => std::process::exit(1),
                    }
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(format!("systemd-run --user -M {}@ {}", user, command))
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .unwrap();
                }
            }
        }
    }

    async fn get_axis_value(&self, event: &InputEvent, deadzone: &i32) -> i32 {
        let distance_from_center: i32 = match self.settings.axis_16_bit {
            false => (event.value() as i32 - 128) * 200,
            _ => event.value() as i32,
        };
        if distance_from_center.abs() <= deadzone * 200 {
            0
        } else {
            (distance_from_center + 2000 - 1) / 2000
        }
    }

    async fn toggle_modifiers(&self, modifier: Event, value: i32, config: &Config) {
        let mut modifiers = self.modifiers.lock().await;
        if config.mapped_modifiers.all.contains(&modifier) {
            match value {
                1 => {
                    modifiers.push(modifier);
                    modifiers.sort();
                    modifiers.dedup();
                }
                0 => modifiers.retain(|&x| x != modifier),
                _ => {}
            }
        }
    }

    async fn released_keys(&self, modifiers: &Vec<Event>, config: &Config) -> Vec<Key> {
        let mut released_keys: Vec<Key> = Vec::new();
        for (_key, hashmap) in config.bindings.remap.iter() {
            if let Some(event_list) = hashmap.get(modifiers) {
                released_keys.extend(event_list);
            }
        }
        released_keys
    }

    async fn change_active_layout(&self) {
        let mut active_layout = self.active_layout.lock().await;
        let active_window = get_active_window(&self.environment, &self.config).await;
        loop {
            if *active_layout == 3 {
                *active_layout = 0
            } else {
                *active_layout += 1
            };
            if let Some(_) = self.config.iter().find(|&x| {
                x.associations.layout == *active_layout && x.associations.client == active_window
            }) {
                break;
            };
        }
        if self.settings.notify_layout_switch {
            let notify = vec![String::from(format!(
                "notify-send -t 500 'Makima' 'Switching to layout {}'",
                *active_layout
            ))];
            self.spawn_subprocess(&notify).await;
        }
    }

    fn update_config(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let active_layout = self.active_layout.lock().await.clone();
            let active_window = get_active_window(&self.environment, &self.config).await;
            let associations = Associations {
                client: active_window,
                layout: active_layout,
            };
            match self.config.iter().find(|&x| x.associations == associations) {
                Some(config) => {
                    let mut current_config = self.current_config.lock().await;
                    *current_config = config.clone();
                }
                None => {
                    self.change_active_layout().await;
                    self.update_config().await;
                }
            };
        })
    }

    pub async fn cursor_loop(&self) {
        let (cursor, sensitivity, activation_modifiers) =
            if self.settings.lstick.function.as_str() == "cursor" {
                (
                    "left",
                    self.settings.lstick.sensitivity,
                    self.settings.lstick.activation_modifiers.clone(),
                )
            } else if self.settings.rstick.function.as_str() == "cursor" {
                (
                    "right",
                    self.settings.rstick.sensitivity,
                    self.settings.rstick.activation_modifiers.clone(),
                )
            } else {
                ("disabled", 0, vec![])
            };
        if sensitivity != 0 {
            while *self.device_is_connected.lock().await {
                {
                    let stick_position = if cursor == "left" {
                        self.lstick_position.lock().await
                    } else if cursor == "right" {
                        self.rstick_position.lock().await
                    } else {
                        break;
                    };
                    if stick_position[0] != 0 || stick_position[1] != 0 {
                        let modifiers = self.modifiers.lock().await;
                        if activation_modifiers.len() == 0 || activation_modifiers == *modifiers {
                            let (x_coord, y_coord) = if self.settings.invert_cursor_axis {
                                (-stick_position[0], -stick_position[1])
                            } else {
                                (stick_position[0], stick_position[1])
                            };
                            let virtual_event_x: InputEvent =
                                InputEvent::new_now(EventType::RELATIVE, 0, x_coord);
                            let virtual_event_y: InputEvent =
                                InputEvent::new_now(EventType::RELATIVE, 1, y_coord);
                            let mut virt_dev = self.virt_dev.lock().await;
                            virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                            virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return;
        }
    }

    pub async fn scroll_loop(&self) {
        let (scroll, sensitivity, activation_modifiers) =
            if self.settings.lstick.function.as_str() == "scroll" {
                (
                    "left",
                    self.settings.lstick.sensitivity,
                    self.settings.lstick.activation_modifiers.clone(),
                )
            } else if self.settings.rstick.function.as_str() == "scroll" {
                (
                    "right",
                    self.settings.rstick.sensitivity,
                    self.settings.rstick.activation_modifiers.clone(),
                )
            } else {
                ("disabled", 0, vec![])
            };
        if sensitivity != 0 {
            while *self.device_is_connected.lock().await {
                {
                    let stick_position = if scroll == "left" {
                        self.lstick_position.lock().await
                    } else if scroll == "right" {
                        self.rstick_position.lock().await
                    } else {
                        break;
                    };
                    if stick_position[0] != 0 || stick_position[1] != 0 {
                        let modifiers = self.modifiers.lock().await;
                        if activation_modifiers.len() == 0 || activation_modifiers == *modifiers {
                            let (x_coord, y_coord) = if self.settings.invert_scroll_axis {
                                (-stick_position[0], -stick_position[1])
                            } else {
                                (stick_position[0], stick_position[1])
                            };
                            let virtual_event_x: InputEvent =
                                InputEvent::new_now(EventType::RELATIVE, 12, x_coord);
                            let virtual_event_y: InputEvent =
                                InputEvent::new_now(EventType::RELATIVE, 11, y_coord);
                            let mut virt_dev = self.virt_dev.lock().await;
                            virt_dev.axis.emit(&[virtual_event_x]).unwrap();
                            virt_dev.axis.emit(&[virtual_event_y]).unwrap();
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(sensitivity)).await;
            }
        } else {
            return;
        }
    }
}
