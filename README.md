# makima

Makima is a daemon for Linux to remap keyboards, mice, controllers and tablets.\
It works on Wayland, X11 and even tty, as it relies on the `evdev` kernel interface.\
Previously only a controller daemon, the scope has now been expanded because I had nothing better to do.

## Features
- Remap keys, buttons or entire combinations to other keys and macros using simple TOML config files, one for each different device.
- Works with keyboards, mice, controllers and any other device that uses input events present inside `/usr/include/linux/input-event-codes.h`.
- Hotplug to connect and disconnect your devices whenever you want.
- Supports wired and Bluetooth connections.
- If you connect a [supported game controller](https://github.com/cyber-sushi/makima/tree/main#tested-controllers), you can control your cursor and scroll through pages using analog sticks, with adjustable sensitivity.
- You can have multiple sets of key bindings that automatically switch based on the active window (only on Hyprland, Sway and X11 currently).

# Index
- [How to use](https://github.com/cyber-sushi/makima/tree/main#how-to-use)
- [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration)
    - [Example config files](https://github.com/cyber-sushi/makima/tree/main/examples)
    - [Application-specific bindings](https://github.com/cyber-sushi/makima/tree/main#application-specific-bindings)
    - [Change bindings](https://github.com/cyber-sushi/makima/tree/main#change-bindings)
        - [Keys](https://github.com/cyber-sushi/makima/tree/main#bindingskeys-and-combinationskeys)
        - [Axis](https://github.com/cyber-sushi/makima/tree/main#bindingsaxis-and-combinationsaxis)
        - [Settings](https://github.com/cyber-sushi/makima/tree/main#settings)
- [Tested controllers](https://github.com/cyber-sushi/makima/tree/main#tested-controllers)
- [Troubleshooting and FAQ](https://github.com/cyber-sushi/makima/tree/main#troubleshooting-and-faq)

## How to use
1. Download the executable from the Releases page or compile it yourself using Cargo.
2. Create a TOML config file inside `~/.config/makima` (or pick one of the [default ones](https://github.com/cyber-sushi/makima/tree/main/examples)) and rename it with the _exact_ name of your device. You can check the name by running `evtest`. If the name includes a `/`, just omit it.
3. Assign your keybindings inside the config file, follow the [Configuration](https://github.com/cyber-sushi/makima/tree/main#configuration) section for more info.
4. Make sure the `makima` executable has permission to be executed as a program. If not, `cd` into the directory of the executable and use `chmod +x makima`. Alternatively, Right Click > Properties > "allow executing as program" or something like that.
5. Make sure that your user has access to event devices. If it doesn't, use `sudo usermod -aG input yourusername` and reboot.
6. Launch Makima and it'll automatically recognize all connected devices that have a corresponding config file inside `~/.config/makima`.
   - To launch Makima, you can use one of these methods:
     - Launch it from your file manager by double clicking.
     - Launch it from terminal by `cd`ing to the directory of the executable, then using `./makima`.
     - Move the executable to a directory that's in PATH, then launch it using `rofi`, `dmenu` or whatever launcher you use. I personally added `~/.local/share/bin` to PATH and put all my executables there.
     - Create a .desktop file and launch it using that.
     - Autostart it from your window manager's config file (usually `exec /path/to/makima`).

## Configuration
You can find some sample config files on this Github; pick one that fits your use case and copy it inside `~/.config/makima`, then edit it to your needs.\
**To associate a config file to an input device, the file name should be identical to that of the device. If your device's name includes a `/`, just omit it.**\
Example: you run `evtest` and see that your Dualshock 4 controller is named `Sony Interactive Entertainment Wireless Controller`; all you have to do is rename your config file `Sony Interactive Entertainment Wireless Controller.toml`.\
All config files will be parsed automatically when `makima` is launched.

### Application-specific bindings
**(HYPRLAND, SWAY AND X11 ONLY)**\
Have you ever wanted to have a different set of macros for each game that you play? Or maybe you want your controller to input Space when you press X, but only when MPV is focused? Then this is exactly what you're looking for!\
To have app-specific config files, just put `::window_class` at the end of their filename, before `.toml`.\
Example: you want your DS4 controller to have a specific set of keybindings for Firefox, name that file `Sony Interactive Entertainment Wireless Controller::firefox.toml`. To retrieve the window class of a specific application, refer to your compositor's documentation, e.g. on Hyprland type `hyprctl clients` in your terminal while that application is open.\
**Note: on Sway, make sure that the `XDG_DESKTOP_SESSION=sway` environment variable is set, or Makima won't be able to use application-specific bindings.**

## Change bindings
The config file is divided into multiple sections:
- `[bindings.keys]`, where you can rebind single keys or buttons to other keys or macros.
- `[bindings.axis]`, where you can rebind axis type events (e.g. mouse scroll wheel, controller D-Pad) to other keys or macros.
- `[combinations.keys]`, where you can rebind a key sequence of modifier+key (e.g. Ctrl+C, Ctrl+Rightclick etc.) to other keys or macros.
- `[combinations.axis]`, where you can rebind an input sequence of modifier+axis event (e.g. Ctrl+Scrollwheel_up) to other keys or macros.
- `[settings]`, where you can configure a few settings.

### \[bindings.keys] and \[combinations.keys]
Example where the Caps Lock and Ctrl keys are switched:
```
[bindings.keys]
KEY_CAPSLOCK = ["KEY_LEFTCTRL"]
KEY_LEFTCTRL = ["KEY_CAPSLOCK"]
```
Example where pressing Caps Lock triggers the Ctrl+C macro:
```
[bindings.keys]
KEY_CAPSLOCK = ["KEY_LEFTCTRL", "KEY_C"]
```
Example where pressing any key on your mouse will immediately shut down your computer if you're focused on a terminal:
```
[bindings.keys]
BTN_LEFT = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
BTN_RIGHT = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
BTN_MIDDLE = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
```
Example where pressing Ctrl + Shift + K will input A:
```
[combinations.keys]
KEY_LEFTCTRL-KEY_LEFTSHIFT.KEY_K = ["KEY_A"]
```
To see all of the available key codes, refer to the file `/usr/include/linux/input-event-codes.h`.\
Remember that keys like Ctrl and Alt will have key codes like `KEY_LEFTCTRL`, `KEY_RIGHTCTRL`, `KEY_LEFTALT` and `KEY_RIGHTALT`. Just using `KEY_CTRL` and `KEY_ALT` will throw a parsing error because the key code does not exist.

Keys that are not explicitly remapped will keep their default functionality.\
If you don't need to remap any key, you can just omit the entire `[bindings.keys]` and `[combinations.keys]` paragraphs.

### \[bindings.axis] and \[combinations.axis]
Example where the mouse scroll wheel will copy and paste:
```
[bindings.axis]
SCROLL_WHEEL_UP = ["KEY_LEFTCTRL", "KEY_C"]
SCROLL_WHEEL_DOWN = ["KEY_LEFTCTRL", "KEY_V"]
```
Example where Ctrl + Alt + Scrollwheel will do random stuff idk I have no more creativity:
```
[combinations.axis]
KEY_LEFTCTRL-KEY_LEFTALT.SCROLL_WHEEL_UP = ["KEY_F5", "KEY_SLASH"]
KEY_LEFTCTRL-KEY_LEFTALT.SCROLL_WHEEL_DOWN = ["KEY_MINUS", "KEY_APOSTROPHE"]
```
**Note: axis names are hardcoded, currently you can use the following:**
- `SCROLL_WHEEL_UP`, `SCROLL_WHEEL_DOWN` - for a mouse's scroll wheel
- `DPAD_UP`, `DPAD_DOWN`, `DPAD_LEFT`, `DPAD_RIGHT` - for a game controller's D-Pad
- `BTN_TL2`, `BTN_TR2` - for a game controller's triggers (on most controllers - but not all - these can be put inside `[bindings.keys]` as well, and it will take priority over `[bindings.axis]`)
  
Events that are not explicitly remapped will keep their default functionality.\
If you don't need to remap any axis event, you can just omit the entire `[bindings.axis]` and `[combinations.axis]` paragraphs.

### \[settings]
There are currently 6 available settings:
- `GRAB_DEVICE` will set if Makima should have exclusivity over the device. If set to `"true"`, no other program will read the original input of the device. If set to `"false"`, both the original input and the remapped input will be read by applications. The event reader won't start if this is not set.
- `CURSOR_STICK` will set if your mouse cursor should be moved using your controller's analog sticks, and which of the two sticks should move your cursor. Can be set to `"left"`, `"right"` or `"none"`. Defaults to `"left"` if not set.
- `SCROLL_STICK` will set if 2D scrolling using your controller's analog sticks should be anabled, and which of the two sticks should move your cursor. Can be set to `"left"`, `"right"` or `"none"`. Defaults to `"right"` if not set. If both `CURSOR_STICK` and `SCROLL_STICK` are set to the same value, `CURSOR_STICK` will have priority.
- `CURSOR_SENSITIVITY` will change the speed of your mouse cursor when it's being moved with an analog stick. Lower value is higher sensitivity, minimum `"1"`, suggested `"6"`. If this isn't set, cursor movement will be disabled.
- `SCROLL_SENSITIVITY` will change the scroll speed when scrolling with an analog stick. Lower value is higher sensitivity, minimum `"1"`, suggested `"6"`. If this isn't set, scrolling will be disabled.
- `SIGNED_AXIS_VALUE` is needed if you're using Xbox controllers and Switch Joy-Cons to properly calibrate the analog stick's sensitivity. Set to `"true"` if you're using those controllers. Can be left out otherwise.

Example settings for a keyboard or mouse, notice that only the `GRAB_DEVICE` setting is needed in this case and you can leave everything else out:
```
[settings]
GRAB_DEVICE = "true"
```
Example settings for a an Xbox 360/One controller:
```
[settings]
CURSOR_SENSITIVITY = "6"
SCROLL_SENSITIVITY = "6"
CURSOR_STICK = "left"
SCROLL_STICK = "right"
GRAB_DEVICE = "false"
SIGNED_AXIS_VALUE = "true"
```
Refer to the sample config files on this Github for more information about controllers.

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
