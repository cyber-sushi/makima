# makima

Makima is a daemon for Linux to remap keyboards, mice, controllers and tablets.\
Does not rely on any graphical stack, will work on X11, Wayland and even tty, as it relies on the `evdev` kernel interface.\
Previously only a controller daemon, the scope has now been extended because I had nothing better to do.

## Features:
- Configure your keybindings through simple TOML config files, one for each device.
- Bind and remap keys/buttons or entire macros, sequences and shortcuts.
- Supports keyboards, mice and any other device that uses input events present inside `/usr/include/linux/input-event-codes.h`.
- Hotplug to connect and disconnect your devices whenever you want.
- Connect and remap as many input devices as you want.
- Supports wired and Bluetooth connections.
- If you connect a [supported game controller](https://github.com/cyber-sushi/makima/tree/main#tested-controllers), you can move your cursor using analog sticks with adjustable sensitivity.
- Written in Rust so it's blazingly fast, I think?

## How to use:
1. Download the executable from the Releases page or compile it yourself using Cargo.
2. Create a TOML config file inside `~/.config/makima` and rename it with the _exact_ name of your device. You can check the name by running `evtest`.
3. Assign your keybindings and macros inside the config file, follow [this documentation](https://github.com/cyber-sushi/makima/tree/main#configuration) for more info.
4. If you're using a [supported game controller](https://github.com/cyber-sushi/makima/tree/main#tested-controllers), you can pick a config file from the 'config examples' folder on this Github and rename it with the _exact_ name of your device.
6. Make sure the `makima` executable has permission to be executed as a program. If not, `cd` into the directory of the executable and use `chmod +x makima`. Alternatively, Right Click > Properties > "allow executing as program" or something like that.
7. Make sure your user has access to event devices. If it doesn't, use `sudo usermod -aG input yourusername`.
8. Launch Makima and it'll automatically recognize all connected devices that have a corresponding config file inside `~/.config/makima`.
   - You can either:
     - Launch it from your file manager by double clicking.
     - Launch it from terminal by `cd`ing to the directory of the executable, then using `./makima`.
     - Move the executable to a directory that's in PATH, then launch it using `rofi`, `dmenu` or whatever launcher you use. I personally added `~/.local/share/bin` to PATH and put all my executables there.
     - Create a .desktop file and launch it using that.
     - Autostart it from your window manager's config file (usually `exec /path/to/makima`).

## Configuration:
You can find some sample config files on this Github.
Pick one that fits your use case and copy it inside `~/.config/makima`, then edit it to your needs.
To associate a config file to an input device, the file name should be identical to that of the device.
For example, if you run `evtest` and see that your Dualshock 4 controller is named "Sony Interactive Entertainment Wireless Controller", then you'll have to name your config file "Sony Interactive Entertainment Wireless Controller.toml".
All config files will be parsed automatically when `makima` is launched.

The config file has three sections, a `[keys]` section, where you'll remap your keys, a `[rel]` section to remap scroll wheels and a `[settings]` section containing a few options.

### \[keys]
Example where the Caps Lock and Ctrl keys are switched:
```
[keys]
KEY_CAPSLOCK = ["KEY_LEFTCTRL"]
KEY_LEFTCTRL = ["KEY_CAPSLOCK"]
```
Example where pressing Caps Lock triggers the Ctrl+C macro:
```
[keys]
KEY_CAPSLOCK = ["KEY_LEFTCTRL", "KEY_C"]
```
Example where pressing any key on your mouse will immediately shut down your computer if you're focused on a terminal:
```
[keys]
BTN_LEFT = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
BTN_RIGHT = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
BTN_MIDDLE = ["KEY_S", "KEY_H", "KEY_U", "KEY_T", "KEY_D", "KEY_O", "KEY_W", "KEY_N", "KEY_SPACE", "KEY_N", "KEY_O", "KEY_W", "KEY_ENTER"]
```
To check all of the available key codes, refer to the file `/usr/include/linux/input-event-codes.h`.\
Remember that keys like Ctrl and Alt will have key codes like `KEY_LEFTCTRL`, `KEY_RIGHTCTRL`, `KEY_LEFTALT` and `KEY_RIGHTALT`. Just using `KEY_CTRL` and `KEY_ALT` will throw a parsing error because the key code does not exist.
Keys that are not explicitly remapped will keep their default functionality.
If you don't need to remap keys, you can just omit the entire `[keys]` paragraph.

### \[rel]
Example where the mouse scroll wheel will zoom in and out of a browser page:
```
[rel]
SCROLL_WHEEL_UP = ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_EQUAL"]
SCROLL_WHEEL_DOWN = ["KEY_LEFTCTRL", "KEY_MINUS"]
```
If you don't need to remap your scroll wheel, just omit the `[rel]` paragraph and it'll fall back to default functionality.

### \[settings]
There are currently 4 available settings:
- `GRAB_DEVICE` will set if makima should have exclusivity over the device. If set to `"true"`, no other program will read the original input of the device. If set to `"false"`, both the original input and the remapped input will be read by applications. The event reader won't start if this is not set.
- `MOVE_MOUSE_WITH_STICK` will set if your mouse cursor should be moved using your controller's analog sticks, and which of the two sticks should move your cursor. Can be set to `"left"`, `"right"` or `"none"`. Defaults to "left" if not set.
- `ANALOG_SENSITIVITY` will change the speed of your mouse cursor when moved through an analog stick. Lower value is higher sensitivity, minimum `"1"`, suggested `"6"`. The analog stick won't be read if this is not set.
- `SIGNED_AXIS_VALUE` is needed if you're using Xbox controllers and Switch Joy-Cons to properly calibrate the analog stick's sensitivity. Set to `"true"` if you're using those controllers. Can be left out otherwise.

Example settings for a keyboard or mouse, notice that only the `GRAB_DEVICE` setting is needed in this case and you can leave everything else out:
```
[settings]
GRAB_DEVICE = "true"
```
Example settings for a an Xbox 360/One controller:
```
[settings]
ANALOG_SENSITIVITY = "6"
MOVE_MOUSE_WITH_STICK =	"left"
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
