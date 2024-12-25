#!/bin/sh


# Define variables
binary_name="makima"
service_name="makima.service"
rules_name="50-makima.rules"


# Check for root
if [ "$EUID" -ne 0 ]; then
    echo "Please run this script with sudo."
    exit
fi


# Function to display help
display_help() {
    echo
    echo "Install $binary_name system wide. Usage: $0 [<username>|-u|-h]"
    echo "Options:"
    echo "  -u           Undo the changes made by the script"
    echo "  -h, --help   Display this help message"
    echo
}


# Function to undo changes
undo_changes() {
    systemctl stop $service_name
    systemctl disable $service_name
    rm /usr/local/bin/$binary_name
    rm /etc/udev/rules.d/$rules_name
    rm /etc/systemd/system/$service_name
    rm /etc/modules-load.d/uinput.conf
    systemctl daemon-reload
    echo "Uninstalled successfully."
}


# Check for arguments
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    display_help
    exit
elif [ "$1" = "-u" ]; then
    undo_changes
    exit
elif [ -n "$1" ]; then
    user_name="$1"
else
    echo "Please provide a username to install $binary_name. Usage: $0 [<username>|-u|-h]"
    exit 1
fi


# Copy the binary, udev rules and create the configuration folder
chmod +x "$binary_name"
cp "$binary_name" /usr/local/bin/
cp "$rules_name" /etc/udev/rules.d/
echo "uinput" > /etc/modules-load.d/uinput.conf
mkdir /home/"$user_name"/.config/makima


# Create the systemd unit file
tee "/etc/systemd/system/$service_name" > /dev/null <<EOF
[Unit]
Description=Makima remapping daemon

[Service]
Type=simple
Environment="MAKIMA_CONFIG=/home/$user_name/.config/makima"
ExecStart=/usr/local/bin/$binary_name
Restart=always
RestartSec=3
User=$user_name
Group=input

[Install]
WantedBy=default.target
EOF


# Reload systemd and enable the service
systemctl daemon-reload
systemctl enable --now "$service_name"


echo "Installed successfully."

