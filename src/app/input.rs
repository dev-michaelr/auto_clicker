use super::{AppContext, AppMessages, exit_on_err};
use anyhow::{Context, Result};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, EventSummary, EventType, InputEvent, KeyCode};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::thread::spawn;

pub fn device_input_handler(mut device: Device, cx: &'static AppContext) {
    spawn(move || {
        loop {
            let device_events =
                exit_on_err(device.fetch_events().context("Failed to fetch events"));

            for event in device_events {
                match event.destructure() {
                    EventSummary::Key(_, key, 1) => {
                        // key pressed down
                        if cx
                            .capturing
                            .compare_exchange(true, false, SeqCst, Relaxed)
                            .is_ok()
                        {
                            // capturing is set to false now!
                            cx.captured_input.store(key.code(), SeqCst);
                            // capture input next press
                            cx.sender.input(AppMessages::CaptureEnd(key));
                            continue;
                        }
                        if key.code() != cx.captured_input.load(SeqCst) {
                            continue;
                        }
                        if cx.toggle.load(SeqCst) {
                            // toggle is enabled so we flip
                            cx.keep_clicking.fetch_not(SeqCst);
                        } else {
                            cx.keep_clicking.store(true, SeqCst);
                        }
                        cx.sender.input(AppMessages::ClickingBegin);
                    }
                    EventSummary::Key(_, key, 0) => {
                        // key up
                        if key.code() != cx.captured_input.load(SeqCst) {
                            continue;
                        }
                        if cx.toggle.load(SeqCst) {
                            // toggle is enabled so we dont turn off
                            continue;
                        }
                        cx.keep_clicking.store(false, SeqCst);
                        cx.sender.input(AppMessages::ClickingBegin);
                    }
                    _ => (),
                };
            }
        }
    });
}

pub fn create_virtual_mouse() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::BTN_LEFT);

    VirtualDevice::builder()
        .context("Failed to create virtual device builder")?
        .name("virtual_mouse")
        .with_keys(&keys)
        .context("Failed to initalize virtual mouse with keys")?
        .build()
        .context("Failed to create virtual mouse")
}

pub fn send_left_click(device: &mut VirtualDevice) -> Result<()> {
    let events = [
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 1), // press
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device
        .emit(&events)
        .context("Couldn't emit left click down event")?;
    let events = [
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 0), // SeqCst
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device
        .emit(&events)
        .context("Couldn't emit left click up event")?;
    Ok(())
}
