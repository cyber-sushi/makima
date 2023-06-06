# makima

Makima is a daemon for Linux to bind your controller's buttons to keyboard and mouse macros.

## Features:
- Configure your keybindings through a simple TOML config file.
- Bind single keys/buttons or entire macros, sequences and shortcuts.
- Supports keyboard keys, mouse buttons and other quirky input events.
- Move your cursor using analog sticks with adjustable sensitivity.
- Hotplug to connect and disconnect your controllers whenever you want.
- Connect multiple controllers at the same time so your little brother can join.
- Supports wired and Bluetooth connections.
- Written in Rust so it's blazingly fast or something.

## How to use:
1. Download the executable from the Releases page or compile it yourself using Cargo.
2. Pick a config file compatible with your controller(s) from the "config examples" folder and put it in `~/.config/makima`, rename it to `config.toml`.
3. Customize the keybindings to your liking. Info about the specific configs is commented inside the config files.
4. Make sure the executable has permission to be executed as a program. If not, `cd` to the directory of the executable and use `chmod +x makima`.
5. Make sure your user has access to event devices. If it doesn't, use `sudo usermod -aG input yourusername`.
6. Launch Makima and it'll automatically recognize all connected controllers.
   - You can either:
     - Launch it from your file manager by double clicking.
     - Launch it from terminal by `cd`ing to the directory of the executable, then using `./makima`.
     - Add it to a directory that's in PATH, then launch it using `rofi`, `dmenu` or whatever launcher you use. I personally added `~/.local/share/bin` to PATH and put all my executables there.
     - Create a .desktop file and launch it from there.
     - Autostart it from your window manager's config file (usually `exec /path/to/makima)
