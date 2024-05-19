# makima

Makima is a daemon for Linux to remap keyboards, mice, controllers and tablets.\
It works on both Wayland and X11 as it relies on the `evdev` kernel interface.

## Features
- Translates keys, buttons or combinations to other keys, sequences or shell commands.
- Devices are remapped individually using simple TOML config files.
- Automatically switch layouts based on the active window (only on Hyprland, Sway and X11 currently).
- Works with keyboards, mice, controllers, tablets and any other device that uses `KEY` input events present inside `/usr/include/linux/input-event-codes.h`.
- Also supports some common `ABS` and `REL` events, like analog stick movements and mouse scroll wheels.
- Supports hot plugging to connect and disconnect devices on the fly.
- Works with wired and Bluetooth devices.
- If you connect a [supported game controller](https://github.com/cyber-sushi/makima/tree/main#tested-controllers), you can scroll or move your cursor using analog sticks, with adjustable sensitivity and deadzone.

# Index
- [Installation](https://github.com/cyber-sushi/makima/tree/main#installation)
    - [Building from source](https://github.com/cyber-sushi/makima/tree/main#building-from-source)
- [Running makima](https://github.com/cyber-sushi/makima/tree/main#running-makima)
- [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration)
    - [Example config files](https://github.com/cyber-sushi/makima/tree/main/examples)
    - [Config file naming](https://github.com/cyber-sushi/makima/tree/main#config-file-naming)
    - [Application-specific bindings](https://github.com/cyber-sushi/makima/tree/main#application-specific-bindings)
    - [Change bindings](https://github.com/cyber-sushi/makima/tree/main#bindings-and-settings)
        - [Remap](https://github.com/cyber-sushi/makima/tree/main#remap)
        - [Commands](https://github.com/cyber-sushi/makima/tree/main#commands)
        - [Settings](https://github.com/cyber-sushi/makima/tree/main#settings)
- [Tested controllers](https://github.com/cyber-sushi/makima/tree/main#tested-controllers)
- [Troubleshooting and FAQ](https://github.com/cyber-sushi/makima/tree/main#troubleshooting-and-faq)

## Installation
To install Makima, you can either download the executable from the [Releases page](https://github.com/cyber-sushi/makima/releases) or you can compile it from source using Cargo.
#### Building from source
1. Install `rustup` using your distro's package manager or refer to the [official docs](https://www.rust-lang.org/tools/install) if your distro doesn't ship `rustup`.
2. Run `rustup default stable` which will automatically install Cargo (Rust's package manager) and the Rust toolchain.
3. Git clone and build with:
```
git clone https://github.com/cyber-sushi/makima
cd makima
cargo build --release
```
Once Cargo is done compiling, you should find Makima's executable inside `~/makima/target/release/`.\
After taking the executable, you can delete Makima's folder.

## Running Makima
Make sure that the executable has permissions to run as a program with `chmod +x makima` or with Right Click > Properties > "allow executing as program" or something like that, depending on your file manager.

There are two recommended ways to execute Makima:
- **Run Makima as root with `sudo -E makima`.**\
Navigate into the directory of the executable and use `sudo -E ./makima`.\
Alternatively, add Makima to a directory that's in `PATH`, possibly `/usr/bin` or `~/.local/bin` and simply use `sudo -E makima` from anywhere.\
_Note: the `-E` argument is necessary because it allows Makima to inherit your user environment instead of the root environment when running with `sudo`._

- **Run Makima as a Systemd service.**\
Move the executable into `/usr/bin`.\
Grab `makima.service` from this repo, edit the `User=` line with your username and make sure that the `DBUS_SESSION_BUS_ADDRESS` variable is the same as your user's.\
Move the file into `/etc/systemd/system`, then run `systemctl daemon-reload`.\
After this, you can start and stop Makima with `systemctl start/stop makima` or you can enable/disable it on startup with `systemctl enable/disable makima`. If you change the config files and you want the changes to take place, restart makima with `systemctl restart makima`.

## Configuration
You can find a bunch of [example config files](https://github.com/cyber-sushi/makima/tree/main/examples) on this repo, either pick one of them or create your own from scratch.\
Makima's config directory defaults to `$HOME/.config/makima` but can be changed through the `MAKIMA_CONFIG` environment variable (if you run Makima as a system service, add it directly to the Systemd unit).

### Config file naming
To associate a config file to an input device, the file name should be identical to that of the device. If your device's name includes a `/`, just omit it.\
_Example: you run `evtest` and see that your Dualshock 4 controller is named `Sony Interactive Entertainment Wireless Controller`. All you have to do is rename your config file to `Sony Interactive Entertainment Wireless Controller.toml`._

All config files will be parsed automatically when `makima` is launched.\
Files that don't end with `.toml` and files that start with `.` (dotfiles) won't be parsed, so you can add a dot at the beginning of the filename to mask them from Makima.

### Application-specific bindings
**Hyprland, Sway and X11 only.**\
To apply a config file only to a specific application, just put `::<window_class>` at the end of their filename, before `.toml`.\
_Example: you want your DS4 controller to have a specific set of keybindings for Firefox, name that file `Sony Interactive Entertainment Wireless Controller::firefox.toml`. Note that Flatpaks will have names like `org.mozilla.firefox`._

To retrieve the window class of a specific application, refer to your compositor's documentation, e.g. on Hyprland type `hyprctl clients` in your terminal while that application is open.

**Note: on Wayland, make sure that the `XDG_CURRENT_DESKTOP` environment variable is set, or Makima won't be able to use application-specific bindings.**

## Bindings and settings
The config file is divided into multiple sections:
- `[remap]`, where you can rebind keys, buttons, combinations and some axis events to other keys, buttons and combinations.
- `[commands]`, where you can rebind keys, buttons, combinations and some axis events to shell commands.
- `[settings]`, where you can configure a few settings.

### **[remap]**
```
#Remap a key to another key
KEY1 = ["KEY2"]

#Remap a key to a key sequence
KEY1 = ["KEY2", "KEY3", "KEY4"]

#Remap a key sequence to a single key
MODIFIER1-MODIFIER2-MODIFIER3-KEY1 = ["KEY1"]

#Remap a key sequence to another key sequence
MODIFIER1-MODIFIER2-MODIFIER3-KEY1 = ["KEY1", "KEY2", "KEY3"]
```

### **[commands]**
```
#Use a key to invoke a shell command
KEY1 = ["command1"]

#Use a key to invoke a list of shell commands
KEY1 = ["command1", "command2", "command3"]

#Use a key sequence to invoke a shell command
MODIFIER1-MODIFIER2-MODIFIER3-KEY1 = ["command1"]

#Use a key sequence to invoke a list of shell commands
MODIFIER1-MODIFIER2-MODIFIER3-KEY1 = ["command1", "command2", "command3"]
```
#### Key names:
You can find the `KEY` names inside `/usr/include/linux/input-event-codes.h`, or launch `evtest` to see the events emitted by your devices.\
Remember that keys like Ctrl and Alt have names like `KEY_LEFTCTRL`, `KEY_LEFTALT` etc. Just using `KEY_CTRL` and `KEY_ALT` will throw a parsing error because the key code does not exist.

#### Modifiers and custom modifiers:
You can use as many modifiers as you want when declaring a binding, but the last key _has_ to be a non-modifier key.

Non-modifier keys (e.g. `KEY_A`) can be set in place of a modifier, automatically changing the behavior of that key: when used in combination with other keys, it will only act as a modifier, but when used alone, it will retain its default functionality, although the input event will be dispatched on key-up instead of key-down.

If you want a non-modifier key to act as a modifier without remapping it for that device (e.g. you need it as a modifier when used in combination with another device), you can add it to the `CUSTOM_MODIFIERS` setting. Refer to the `[settings]` section for more info.

#### Modifiers across multiple devices:
Keep in mind that if you want to use modifiers across multiple devices (e.g. `KEY_LEFTCTRL` on your keyboard and `BTN_RIGHT` on your mouse), both devices will have to be read by Makima and thus both will need a config file, even if empty. Having a config file is just a way to tell Makima "Hey, read this device!".

#### Mixed bindings:
When declaring a binding, you can put a dash in front of the key on the left side to tell Makima that when you press that key, it should mix the output command with whatever other modifier is pressed.

This means that, for example, you can bind `BTN_SELECT-BTN_TL2 = ["KEY_LEFTALT"]` and `-BTN_TR2 = ["KEY_TAB"]` to simulate the Alt-Tab sequence: press the buttons in the first binding, and then tap `TR2` to advance in the Alt-Tab menu.

If pressed alone, it will just emit designated event.

#### Axis events:
Axis events such as scroll wheels and analog stick movements are hardcoded, currently you can use the following:
- `SCROLL_WHEEL_UP`, `SCROLL_WHEEL_DOWN` - for a mouse's scroll wheel
- `BTN_DPAD_UP`, `BTN_DPAD_DOWN`, `BTN_DPAD_LEFT`, `BTN_DPAD_RIGHT` - for a game controller's D-Pad
- `BTN_TL2`, `BTN_TR2` - for a game controller's triggers
- `LSTICK_UP`, `LSTICK_DOWN`, `LSTICK_LEFT`, `LSTICK_RIGHT`, `RSTICK_UP`, `RSTICK_DOWN`, `RSTICK_LEFT`, `RSTICK_RIGHT` - for a game controller's analog sticks

Refer to the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) for more information.


### \[settings]
#### `GRAB_DEVICE`
Sets if Makima should have exclusivity over the device.\
If `"true"`, no other program will read the original input of the device. If `"false"`, both the original input and the remapped input will be read by applications.
#### `LSTICK` and `RSTICK`
Set the function of the left and right analog sticks, respectively.\
`"bind"` will make them available for rebinding in `[remap]` and `[commands]`, `"cursor"` will use them to move your mouse cursor, `"scroll"` will use them to scroll, `"disabled"` will disable them.
#### `LSTICK_SENSITIVITY` and `RSTICK_SENSITIVITY`
Set the sensitivity of your left and right analog sticks when using them to scroll or move your cursor.\
Lower value is higher sensitivity, minimum `"1"`, suggested `"6"`. If this is set to `"0"` or if it's not set, cursor movement and scroll will be disabled.
#### `LSTICK_DEADZONE` and `RSTICK_DEADZONE`
Set how much your analog sticks should be tilted before their inputs are detected.\
Particularly useful for older devices that suffer from drifting. Use a value between `"0"` and `"128"`.
#### `LSTICK_ACTIVATION_MODIFIERS` and `RSTICK_ACTIVATION_MODIFIERS`
When using analog sticks in `cursor` or `scroll` mode, normally, they're always active. However, if you specify a list of keys or modifiers in `LSTICK_ACTIVATION_MODIFIERS` or `RSTICK_ACTIVATION_MODIFIERS`, they'll only be active when the modifiers are pressed.\
Example:
```
#only move the cursor when select and start are pressed
LSTICK = "cursor"
LSTICK_ACTIVATION_MODIFIERS = "BTN_SELECT-BTN_START"
```
#### `16_BIT_AXIS`
This is needed if you're using Xbox controllers and Switch Joy-Cons to properly calibrate the analog stick's sensitivity.\
Set to `"true"` if you're using those controllers.
#### `CUSTOM_MODIFIERS`
The keys listed in this parameter will change their behavior to act as modifiers.\
While pressed, they will act as modifiers without emitting their respective `KEY` event, possibly changing the behavior of other keys if specified in `[remap]`. On release, they will emit their default `KEY` event only if no other keystroke happened while they were pressed.\
This is useful if you want to have a key that behaves like a modifier but can still emit its own event if pressed alone.\
You can list multiple keys to treat as modifiers with the following syntax:\
`CUSTOM_MODIFIERS = "KEY_A-KEY_BACKSLASH-KEY_GRAVE"`

Refer to the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) for more information.

## Tested controllers
- DualShock 2
- DualShock 3
- DualShock 4
- DualSense
- Xbox 360
- Xbox One
- Xbox Elite 2
- Stadia
- Switch Joy-Cons

To add other controllers, please open an issue.

## Troubleshooting and FAQ
**Q**: My device actually shows as three different devices in evtest, do I need to create three different config files, one for each device?\
**A**: Each device will have a certain set of features, e.g. a DS4 controller is recognized as a touchpad, a motion sensor and a controller. A mouse is usually recognized as a mouse and a keyboard (for the additional keys). Just create a config file for the devices/features that you need to remap, and ignore the others.

**Q**: My controller works when using Bluetooth but not when using wired connection or vice-versa, why?\
**A**: Some devices have a different evdev name when connected through Bluetooth, for example a `Sony Interactive Entertainment Wireless Controller` is just seen as `Wireless Controller` when connected via Bluetooth. You'll need to create a copy of the config file with that name.

**Q**: Will application-specific bindings be implemented for desktops other than Hyprland, Sway and X11?\
**A**: Gnome on Wayland requires an extension to retrieve the active window through D-Bus (???) and KDE on Wayland requires to use JavaScript plug-ins to make any request to KWin (also ???), which is why I haven't implemented active window tracking for them. If anyone finds a better solution, I'm all for it. Regarding other compositors, feel free to open an issue and I'll look into it.

**Q**: Makima gives me a "Permission Denied" error when launching, what do I do?\
**A**: Make sure that the `uinput` kernel module is loaded. You can load it with `sudo modprobe uinput`. To make it permanent, create `/etc/modules-load.d/uinput.conf` and write `uinput` inside.

**Q**: Flatpak applications don't start when launched through Makima.\
**A**: When running as a Systemd service, Makima doesn't communicate with desktop portals so it's unable to launch Flatpaks. Currently looking for a solution.

**Q**: SELinux prevents Makima's system service from running, what do I do?\
**A**: Put `makima.service` inside `/usr/lib/systemd/system` instead of `/etc/systemd/system`, then run the following commands:
- `sudo semanage fcontext -a -t bin_t "/usr/lib/systemd/system/makima.service"`
- `sudo restorecon -v /usr/lib/systemd/system/makima.service`
- `sudo semanage fcontext -a -t bin_t "/usr/bin/makima"`
- `sudo restorecon -v /usr/bin/makima`
