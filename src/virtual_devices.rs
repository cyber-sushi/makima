use evdev::uinput::VirtualDevice;


pub struct VirtualDevices {
    pub keys: VirtualDevice,
    pub relative_axes: VirtualDevice,
}

impl VirtualDevices {
    pub fn new(keys: VirtualDevice, relative_axes: VirtualDevice) -> Self {
        Self {
            keys: keys,
            relative_axes: relative_axes,
        }
    }
}

