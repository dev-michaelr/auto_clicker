# About

A lightweight autoclicker for Linux supporting both X11 and Wayland. Unlike traditional autoclickers that hook into a display server, it works at the **kernel input level** via `/dev/input` — meaning it works anywhere: games, Wayland compositors, fullscreen apps, and terminals alike.

Configure everything through a simple TOML config file. Set your click interval down to millisecond precision, bind a mouse side button or any key as your toggle hotkey, and manually specify your input devices by their `/dev/input` ID.

> **Beta:** This is early software. Expect rough edges.

<img width="1043" height="681" alt="Image" src="https://github.com/user-attachments/assets/41e1080e-793d-4ed9-bafb-64ce2518802c" />

## Features

- X11 and Wayland compatible — no display server dependency
- Millisecond-precision click intervals
- Toggle or hold mode
- Hotkey support for mouse buttons and keyboard keys
- Minimal config — one TOML file, no GUI needed

## Installation

### Prerequisites

- Rust (install via https://rustup.rs)

### Setup

Reading from `/dev/input` requires your user to be in the `input` group:

```bash
sudo usermod -aG input $USER
```

Then log out and back in for the group change to take effect.

### Build

```bash
git clone https://github.com/dev-michaelr/auto_clicker.git
cd auto_clicker
cargo install --path .
```

The binary will located at `~/.cargo/bin/auto_clicker`.

### Run

```bash
auto_clicker
```

Make sure `~/.cargo/bin` is in your path, you can check your path with:

```bash
    echo $PATH
```

## Config

Create a config file and fill in your own device paths. You can find them with:

```bash
ls /dev/input/by-id/
```

You will want the one ending with event-mouse for mouse and event-kbd for keyboard.

Location: `~/.config/auto_clicker/config.toml`

```toml
interval = "15ms"
hotkey = "BTN_EXTRA"
toggle = false

[devices]
mouse = "/dev/input/by-id/example-mouse"
keyboard = "/dev/input/by-id/example-keyboard"
```
