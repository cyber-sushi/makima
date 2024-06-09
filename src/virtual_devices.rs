use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    Key,
};

pub struct VirtualDevices {
    pub keys: VirtualDevice,
    pub axis: VirtualDevice,
}

impl VirtualDevices {
    pub fn new() -> Self {
        let mut key_capabilities = evdev::AttributeSet::new();
        for i in 1..334 {
            key_capabilities.insert(Key(i));
        }
        let mut axis_capabilities = evdev::AttributeSet::new();
        for i in 0..13 {
            axis_capabilities.insert(evdev::RelativeAxisType(i));
        }
        let keys_builder = VirtualDeviceBuilder::new()
            .expect("Unable to create virtual device through uinput. Take a look at the Troubleshooting section for more info.")
            .name("Makima Virtual Keyboard/Mouse")
            .with_keys(&key_capabilities).unwrap();
        let axis_builder = VirtualDeviceBuilder::new()
            .expect("Unable to create virtual device through uinput. Take a look at the Troubleshooting section for more info.")
            .name("Makima Virtual Pointer")
            .with_relative_axes(&axis_capabilities).unwrap();
        let virtual_device_keys = keys_builder.build().unwrap();
        let virtual_device_axis = axis_builder.build().unwrap();
        Self {
            keys: virtual_device_keys,
            axis: virtual_device_axis,
        }
    }
}
