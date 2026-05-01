# About

A lightweight autoclicker for Linux supporting both X11 and Wayland. Unlike traditional autoclickers that hook into a display server, it works at the **kernel input level** via `/dev/input` meaning it works anywhere: games, any compositors, fullscreen apps.

> **Beta:** This is early software. Expect rough edges.

<img width="1043" height="681" alt="Image" src="https://github.com/user-attachments/assets/41e1080e-793d-4ed9-bafb-64ce2518802c" />

## Features

- X11 and Wayland compatible
- Blazingly fast low latency clicking
- Millisecond-precision click intervals
- Toggle or hold mode
- Hotkey support for mouse buttons and keyboard keys
- Minimal config

## Installation

### Download
[Latest release](https://github.com/dev-michaelr/auto_clicker/releases/latest)

You need to make it executable after downloading through your file manager or command:

```bash
chmod +x auto_clicker
```

### Dependencies
- gtk4 (`libgtk-4-1` on Debian/Ubuntu, `gtk4` on Arch, etc.)

### Setup

Read and write access to `/dev/input` requires your user to be in the `input` group:

```bash
sudo usermod -aG input $USER
```

Then log out and back in for the group change to take effect.

### Building from source

#### Prerequisites
- Have rust installed https://rustup.rs/
- Have git installed https://git-scm.com/

```bash
cargo install --git https://github.com/dev-michaelr/auto_clicker.git --tag v0.1.2
```

The binary will located at `~/.cargo/bin/auto_clicker`.

Make sure `~/.cargo/bin` is in your path. You can check your path with:

```bash
echo $PATH
```

## Config

Before you run `auto_clicker`, create a config file located at `~/.config/auto_clicker/config.toml` and fill in your own device paths. You can find them with:
```bash
ls /dev/input/by-id/
```

You will want devices ending with event-mouse for your mouse and event-kbd for your keyboard.

Example:

```toml
interval = "15ms"
hotkey = "BTN_EXTRA"
toggle = false

[devices]
mouse = "/dev/input/by-id/example-event-mouse"
keyboard = "/dev/input/by-id/example-event-kbd"
```
