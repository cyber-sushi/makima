use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    Key,
};

pub struct VirtualDevices {
    pub keys: VirtualDevice,
    pub axis: VirtualDevice,
    pub abs: VirtualDevice,
}

impl VirtualDevices {
    pub fn new(device: evdev::Device) -> Self {
        let mut key_capabilities = evdev::AttributeSet::new();
        for i in 1..334 {
            key_capabilities.insert(Key(i));
        }
        let mut axis_capabilities = evdev::AttributeSet::new();
        for i in 0..13 {
            axis_capabilities.insert(evdev::RelativeAxisType(i));
        }
        let mut tablet_abs_capabilities: Vec<evdev::UinputAbsSetup> = Vec::new();
        if let Ok(absinfo) = device.get_abs_state() {
            for (axis_type, info) in absinfo.iter().enumerate() {
                if [0, 1, 2, 5, 6, 8, 24, 25, 26, 27].contains(&axis_type) {
                    let new_absinfo = evdev::AbsInfo::new(
                        info.value,
                        info.minimum,
                        info.maximum,
                        info.fuzz,
                        info.flat,
                        info.resolution,
                    );
                    tablet_abs_capabilities.push(evdev::UinputAbsSetup::new(
                        evdev::AbsoluteAxisType(axis_type.try_into().unwrap()),
                        new_absinfo,
                    ))
                }
            }
        }
        let mut tablet_capabilities = evdev::AttributeSet::new();
        for i in 272..277 {
            tablet_capabilities.insert(evdev::Key(i));
        }
        for i in 320..325 {
            tablet_capabilities.insert(evdev::Key(i));
        }
        for i in 326..328 {
            tablet_capabilities.insert(evdev::Key(i));
        }
        for i in 330..333 {
            tablet_capabilities.insert(evdev::Key(i));
        }
        let mut tab_rel = evdev::AttributeSet::new();
        tab_rel.insert(evdev::RelativeAxisType(8));
        let mut tab_msc = evdev::AttributeSet::new();
        tab_msc.insert(evdev::MiscType(0));
        let mut pointer_prop = evdev::AttributeSet::new();
        pointer_prop.insert(evdev::PropType::POINTER);
        let keys_builder = VirtualDeviceBuilder::new()
            .expect("Unable to create virtual device through uinput. Take a look at the Troubleshooting section for more info.")
            .name("Makima Virtual Keyboard/Mouse")
            .with_keys(&key_capabilities).unwrap();
        let axis_builder = VirtualDeviceBuilder::new()
            .expect("Unable to create virtual device through uinput. Take a look at the Troubleshooting section for more info.")
            .name("Makima Virtual Pointer")
            .with_relative_axes(&axis_capabilities).unwrap();
        let mut abs_builder = VirtualDeviceBuilder::new()
            .expect("Unable to create virtual device through uinput. Take a look at the Troubleshooting section for more info.")
            .name("Makima Virtual Pen/Tablet")
            .with_properties(&pointer_prop).unwrap()
            .with_msc(&tab_msc).unwrap()
            .with_relative_axes(&tab_rel).unwrap()
            .with_keys(&tablet_capabilities).unwrap()
            .input_id(device.input_id());
        for abs_setup in tablet_abs_capabilities {
            abs_builder = abs_builder.with_absolute_axis(&abs_setup).unwrap();
        }
        let virtual_device_keys = keys_builder.build().unwrap();
        let virtual_device_axis = axis_builder.build().unwrap();
        let virtual_device_abs = abs_builder.build().unwrap();
        Self {
            keys: virtual_device_keys,
            axis: virtual_device_axis,
            abs: virtual_device_abs,
        }
    }
}
