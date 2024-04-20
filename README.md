# makima

Makima is a daemon for Linux to remap keyboards, mice, controllers and tablets.\
It works on Wayland, X11 and even tty, as it relies on the `evdev` kernel interface.\
Previously only a controller daemon, the scope has now been expanded because I had nothing better to do.

## Features
- Remap keys, buttons or entire combinations to other keys, sequences or shell commands using simple TOML config files, one for each different device.
- Works with keyboards, mice, controllers and any other device that uses `KEY` input events present inside `/usr/include/linux/input-event-codes.h`, and also supports common `ABS` and `REL` events.
- Hotplug to connect and disconnect your devices whenever you want.
- Supports wired and Bluetooth connections.
- If you connect a [supported game controller](https://github.com/cyber-sushi/makima/tree/main#tested-controllers), you can move your cursor or scroll through pages using analog sticks, with adjustable sensitivity and deadzone.
- You can have multiple sets of key bindings that automatically switch based on the active window (only on Hyprland, Sway and X11 currently).

# Index
- [Installation](https://github.com/cyber-sushi/makima/tree/main#installation)
    - [Building from source](https://github.com/cyber-sushi/makima/tree/main#building-from-source)
    - [Config files](https://github.com/cyber-sushi/makima/tree/main#config-files)
- [Running makima](https://github.com/cyber-sushi/makima/tree/main#running-makima)
- [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration)
    - [Example config files](https://github.com/cyber-sushi/makima/tree/main/examples)
    - [Config file naming](https://github.com/cyber-sushi/makima/tree/main#config-file-naming)
    - [Application-specific bindings](https://github.com/cyber-sushi/makima/tree/main#application-specific-bindings)
    - [Change bindings](https://github.com/cyber-sushi/makima/tree/main#bindings-and-settings)
        - [Remap](https://github.com/cyber-sushi/makima/tree/main#remap)
        - [Commands](https://github.com/cyber-sushi/makima/tree/main#commands)
        - [Settings](https://github.com/cyber-sushi/makima/tree/main#settings)
- [Desktop integration](https://github.com/cyber-sushi/makima/tree/main#desktop-integration)
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

#### Config files
Makima's config directory defaults to `~/.config/makima` but can be changed through the `MAKIMA_CONFIG` environment variable.\
You can pick one of the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) and copy it inside Makima's config directory, or make your own from scratch.\
You can find everything about config file naming and configuration in the [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration) section.

## Running Makima
#### Executable permissions
Make sure that the executable has permissions to be executed as a program with `chmod +x makima` or with Right Click > Properties > "allow executing as program" or something like that, depending on your file manager.
#### Evdev permissions
In order to work properly, Makima needs access to the `evdev` kernel module which contains event devices.\
To do so, you can do **one of the following**:
- **Use `sudo usermod -aG input username` and reboot.**\
_Note: some users might not like this one because it makes all applications potentially able to read your inputs._
- **Run Makima as root.**\
_Note: if you use Makima to launch shell commands, some of them might not work, e.g. using `notify-send` will send notifications to your root user instead of your normal user. To work around this, you can launch your commands with `su - username -c 'command'`, which might nor might not work. I'll address this at some point._
#### Usage
To run Makima, you can just `cd` to its directory and use `./makima` or you can double-click it in your file manager to make it start in the background.\
If you want to integrate it with your desktop experience, you can take a look at the [Desktop integration](https://github.com/cyber-sushi/makima/tree/main#desktop-integration) section.

## Configuration

### Config file naming
To associate a config file to an input device, the file name should be identical to that of the device. If your device's name includes a `/`, just omit it.\
Example: you run `evtest` and see that your Dualshock 4 controller is named `Sony Interactive Entertainment Wireless Controller`; all you have to do is rename your config file `Sony Interactive Entertainment Wireless Controller.toml`.\
All config files will be parsed automatically when `makima` is launched.\
Files that don't end with `.toml` and files that start with `.` (dotfiles) won't be parsed, so you can add a dot at the beginning of the filename to mask them from Makima.

### Application-specific bindings
**Hyprland, Sway and X11 only.**\
Have you ever wanted to have a different set of bindings for each game or application? Then this is exactly what you're looking for!\
To apply a config file only to a specific application, just put `::<window_class>` at the end of their filename, before `.toml`.\
Example: you want your DS4 controller to have a specific set of keybindings for Firefox, name that file `Sony Interactive Entertainment Wireless Controller::firefox.toml`.\
To retrieve the window class of a specific application, refer to your compositor's documentation, e.g. on Hyprland type `hyprctl clients` in your terminal while that application is open.\
**Note: on Sway, make sure that the `XDG_DESKTOP_SESSION=sway` environment variable is set, or Makima won't be able to use application-specific bindings.**

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
You can use as many modifiers as you want when declaring a binding, but the last key _has_ to be a non-modifier key.\
Additionally, you can set a non-modifier key (e.g. `KEY_A`) in place of a modifier, which will automatically change the behavior of that key: when used in combination with other keys, it will only act as a modifier, but when used alone, it will retain its default functionality, although the input event will be dispatched on key-up instead of key-down.
If you want a non-modifier key to act as a modifier without remapping it for that device (e.g. you need it as a modifier when used in combination with another device), you can add it to the `CUSTOM_MODIFIERS` setting. Refer to the `[settings]` section for more info.

#### Modifiers across multiple devices:
Keep in mind that if you want to use modifiers across multiple devices (e.g. `KEY_LEFTCTRL` on your keyboard and `BTN_RIGHT` on your mouse), both devices will have to be read by Makima and thus both will need a config file, even if empty. Having a config file is just a way to tell Makima "Hey, read this device!".

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

## Desktop integration
There are multiple ways of running Makima and integrating it with your desktop experience, I'll give some examples.
- Launch it from your file manager, add it to your desktop or create a link.\
_This will generally just launch Makima in the background._
- Launch it from terminal by `cd`ing to the directory of the executable, then using `./makima`.\
_Useful because you get a lot of diagnostics in case something doesn't work._
- Move the executable to a directory that's in PATH, then launch it using `rofi`, `dmenu` or whatever launcher you use.\
_Most people add `~/.local/share/bin` to PATH and put all their executable files there._
- Create a .desktop file for Makima and put it inside `~/.local/share/applications`.\
_This will add Makima to your DE's app drawer or app menu, and will make it visible in `rofi`, `wofi` etc when used in `drun` mode._\
- Autostart it from your window manager's config file.\
_Most window managers and Wayland compositors have a way to start applications from their config file, like `exec /path/to/makima` (Sway) or `exec-once = /path/to/makima` (Hyprland)._
- Create a systemd service for Makima.\
_This will let you start/stop Makima using `systemctl start/stop makima` and enable/disable on startup it using `systemctl enable/disable makima`._

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
**A**: If someone requests it, I might look into it.

**Q**: Makima gives me a "Permission Denied" error when launching, what do I do?\
**A**: If you're certain that you've correctly added your user to the `input` group through `sudo usermod -aG input yourusername` and rebooted (you can verify it by running `groups` and see if it returns `input`), then maybe the `uinput` kernel module isn't loaded. You can load it with `sudo modprobe uinput`. To make it permanent, create `/etc/modules-load.d/uinput.conf` and write `uinput` inside.
