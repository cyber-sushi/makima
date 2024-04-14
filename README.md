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
- [How to use](https://github.com/cyber-sushi/makima/tree/main#how-to-use)
- [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration)
    - [Example config files](https://github.com/cyber-sushi/makima/tree/main/examples)
    - [Config file naming](https://github.com/cyber-sushi/makima/tree/main#config-file-naming)
    - [Application-specific bindings](https://github.com/cyber-sushi/makima/tree/main#application-specific-bindings)
    - [Change bindings](https://github.com/cyber-sushi/makima/tree/main#change-bindings)
        - [Remap](https://github.com/cyber-sushi/makima/tree/main#remap)
        - [Commands](https://github.com/cyber-sushi/makima/tree/main#commands)
        - [Settings](https://github.com/cyber-sushi/makima/tree/main#settings)
- [Tested controllers](https://github.com/cyber-sushi/makima/tree/main#tested-controllers)
- [Troubleshooting and FAQ](https://github.com/cyber-sushi/makima/tree/main#troubleshooting-and-faq)

## How to use
1. Download the executable from the Releases page or compile it yourself using Cargo.
2. Create a TOML config file inside `~/.config/makima` (or pick one of the [default ones](https://github.com/cyber-sushi/makima/tree/main/examples)) and rename it with the _exact_ name of your device. You can check the name by running `evtest`. If the name includes a `/`, just omit it.
3. Assign your keybindings inside the config file, follow the [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration) section for more info.
4. Make sure the `makima` executable has permission to be executed as a program. If not, `cd` into the directory of the executable and use `chmod +x makima`. Alternatively, Right Click > Properties > "allow executing as program" or something like that.
5. Make sure that your user has access to event devices. If it doesn't, use `sudo usermod -aG input yourusername` and reboot.
6. Launch Makima and it'll automatically recognize all connected devices that have a corresponding config file inside `~/.config/makima`. To launch Makima, you can use one of these methods:
     - Launch it from your file manager by double clicking.
     - Launch it from terminal by `cd`ing to the directory of the executable, then using `./makima`.
     - Move the executable to a directory that's in PATH, then launch it using `rofi`, `dmenu` or whatever launcher you use. I personally added `~/.local/share/bin` to PATH and put all my executables there.
     - Create a .desktop file and launch it using that.
     - Autostart it from your window manager's config file (usually `exec /path/to/makima` or `exec-once = /path/to/makima`).

## Configuration
You can pick one of the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) and copy it inside `~/.config/makima`, rename it and edit it to your needs.

### Config file naming
**To associate a config file to an input device, the file name should be identical to that of the device. If your device's name includes a `/`, just omit it.**\
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

## Change bindings
The config file is divided into multiple sections:
- `[remap]`, where you can rebind keys, buttons, combinations and some axis events to other keys, buttons and combinations.
- `[commands]`, where you can rebind keys, buttons, combinations and some axis events to shell commands.
- `[settings]`, where you can configure a few settings.

**Base syntax:**

### **[remap]**
```
#Remap a key to another key
KEY1 = ["KEY2"]

#Remap a key to a key sequence
KEY1 = ["KEY2", "KEY3", "KEY4"]

#Remap a key sequence (Ctrl/Alt/Shift/Meta + Key) to another key
MODIFIER1-MODIFIER2-MODIFIER3.KEY1 = ["KEY1"]

#Remap a key sequence (Ctrl/Alt/Shift/Meta + Key) to a key sequence
MODIFIER1-MODIFIER2-MODIFIER3.KEY1 = ["KEY1", "KEY2", "KEY3"]
```

### **[commands]**
```
#Use a key to invoke a shell command
KEY1 = ["command1"]

#Use a key to invoke a list of shell commands
KEY1 = ["command1", "command2", "command3"]

#Use a key sequence (Ctrl/Alt/Shift/Meta + Key) to invoke a shell command
MODIFIER1-MODIFIER2-MODIFIER3.KEY1 = ["command1"]

#Use a key sequence (Ctrl/Alt/Shift/Meta + Key) to invoke a list of shell commands
MODIFIER1-MODIFIER2-MODIFIER3.KEY1 = ["command1", "command2", "command3"]
```
You can find the `KEY` names inside `/usr/include/linux/input-event-codes.h`, or launch `evtest` to see the events emitted by your devices.\
Remember that keys like Ctrl and Alt have names like `KEY_LEFTCTRL`, `KEY_LEFTALT` etc. Just using `KEY_CTRL` and `KEY_ALT` will throw a parsing error because the key code does not exist.\
Keys that are not explicitly remapped will keep their default functionality.

**Note: axis events such as scroll wheels and analog stick movements are hardcoded, currently you can use the following:**
- `SCROLL_WHEEL_UP`, `SCROLL_WHEEL_DOWN` - for a mouse's scroll wheel
- `DPAD_UP`, `DPAD_DOWN`, `DPAD_LEFT`, `DPAD_RIGHT` - for a game controller's D-Pad
- `BTN_TL2`, `BTN_TR2` - for a game controller's triggers
- `LSTICK_UP`, `LSTICK_DOWN`, `LSTICK_LEFT`, `LSTICK_RIGHT`, `RSTICK_UP`, `RSTICK_DOWN`, `RSTICK_LEFT`, `RSTICK_RIGHT`, - for a game controller's analog sticks

Refer to the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) for more information.


### \[settings]
- `GRAB_DEVICE` sets if Makima should have exclusivity over the device. _If set to `"true"`, no other program will read the original input of the device. If set to `"false"`, both the original input and the remapped input will be read by applications. The event reader won't start if this is not set._
- `LSTICK` and `RSTICK` set the function of the left and right analog sticks, respectively. _`"bind"` will make them available for rebinding in `[remap]` and `[commands]`, `"cursor"` will use them to move your mouse cursor, `"scroll"` will use them to scroll, `"disabled"` will disable them._
- `LSTICK_SENSITIVITY` and `RSTICK_SENSITIVITY` set the sensitivity of your left and right analog sticks when using them to scroll or move your cursor. _Lower value is higher sensitivity, minimum `"1"`, suggested `"6"`. If this is set to `"0"` or if it's not set, cursor movement and scroll will be disabled._
- `LSTICK_DEADZONE` and `RSTICK_DEADZONE` set how much your analog sticks should be tilted before their inputs are detected. _Particularly useful for older devices that suffer from drifting. Use a value between `"0"` and `"128"`._
- `16_BIT_AXIS` is needed if you're using Xbox controllers and Switch Joy-Cons to properly calibrate the analog stick's sensitivity. _Set to `"true"` if you're using those controllers._

**Note: only the `GRAB_DEVICE` setting is mandatory, everything else can be left out if not needed.**

Refer to the [sample config files](https://github.com/cyber-sushi/makima/tree/main/examples) for more information.

## Tested controllers:
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

## Troubleshooting and FAQ:
**Q**: My device actually shows as three different devices in evtest, do I need to create three different config files, one for each device?\
**A**: Each device will have a certain set of features, e.g. a DS4 controller is recognized as a touchpad, a motion sensor and a controller. A mouse is usually recognized as a mouse and a keyboard (for the additional keys). Just create a config file for the devices/features that you need to remap, and ignore the others.

**Q**: Can I map a key sequence (e.g. Ctrl+C) to something else?\
**A**: Yes! Since version 0.4.0, you can remap key modifiers (Ctrl, Shift, Alt, Meta) + key, to call another key or macro.

**Q**: My controller works when using Bluetooth but not when using wired connection or vice-versa, why?\
**A**: Some devices have a different evdev name when connected through Bluetooth, for example a `Sony Interactive Entertainment Wireless Controller` is just seen as `Wireless Controller` when connected via Bluetooth. You'll need to create a copy of the config file with that name.

**Q**: Will application-specific bindings be implemented for desktops other than Hyprland, Sway and X11?\
**A**: If someone requests it, I might look into it.

**Q**: Makima gives me a "Permission Denied" error when launching, what do I do?\
**A**: If you're certain that you've correctly added your user to the `input` group through `sudo usermod -aG input yourusername` and rebooted (you can verify it by running `groups` and see if it returns `input`), then maybe the `uinput` kernel module isn't loaded. You can load it with `sudo modprobe uinput`. To make it permanent, create `/etc/modules-load.d/uinput.conf` and write `uinput` inside.
