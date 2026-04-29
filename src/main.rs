use evdev::KeyCode;
use evdev::uinput::VirtualDevice;
use evdev::*;
use gtk::prelude::*;
use humantime::format_duration;
use relm4::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64};
use std::sync::{Arc, mpsc};
use std::thread::{sleep, spawn};
use std::time::Duration;

const MIN_DURATION: Duration = Duration::from_millis(1);

#[derive(Serialize, Deserialize)]
struct Devices {
    mouse: PathBuf,
    keyboard: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(with = "humantime_serde", default = "default_interval")]
    interval: Duration,

    #[serde(default = "default_hotkey")]
    hotkey: KeyCode,

    #[serde(default)]
    toggle: bool,

    devices: Devices,
}

fn default_hotkey() -> KeyCode {
    KeyCode::BTN_EXTRA
}

fn default_interval() -> Duration {
    Duration::from_millis(15)
}

fn config_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(".config/auto_clicker/config.toml")
}

#[derive(Debug)]
enum AppDuration {
    Milliseconds = 0,
    Seconds = 1,
    Minutes = 2,
    Hours = 3,
}

struct AppModel {
    durations: [Duration; 4],
    capturing: bool,
    is_clicking: bool,
    config: Config,
    click_sender: mpsc::Sender<()>,
    cx: AppContext,
    dirty: bool,
}

#[derive(Clone)]
struct AppContext {
    toggle: Arc<AtomicBool>,
    keep_clicking: Arc<AtomicBool>,
    captured_input: Arc<AtomicU16>,
    capturing: Arc<AtomicBool>,
    duration: Arc<AtomicU64>,
    sender: ComponentSender<AppModel>,
}

struct DeviceContext {
    device: Device,
    cx: AppContext,
}

#[derive(Debug)]
enum AppMessages {
    CaptureBegin,
    ClickingBegin,
    CaptureEnd(KeyCode),
    ClickingEnd,
    Toggle(bool),
    DurationChanged(AppDuration, Duration),
}

impl AppModel {
    fn key_label(&self) -> String {
        match self.capturing {
            true => String::from("Press any key..."),
            false => format!("{:?}", self.config.hotkey),
        }
    }
    fn save_config(&mut self) {
        if !self.dirty {
            return;
        }
        let path = config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let contents = toml::to_string(&self.config).unwrap();
        std::fs::write(path, contents).unwrap();
        self.dirty = false;
    }
}

fn device_input_handler(mut device_context: DeviceContext) {
    let cx = device_context.cx;

    spawn(move || {
        loop {
            let device_events = device_context.device.fetch_events().unwrap();
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

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = Config;
    type Input = AppMessages;
    type Output = ();

    view! {
        gtk::Window {
            set_default_size: (100,100),

            #[watch]
            set_can_target: !( model.capturing || model.is_clicking ),

            gtk::Box {
                set_orientation : gtk::Orientation::Vertical,
                set_margin_all:6,
                set_spacing:12,
                gtk::Box {
                    set_orientation : gtk::Orientation::Vertical,
                    set_spacing:12,
                    gtk::Label {
                        #[watch]
                        set_markup: &format!("<b>Click Interval: {}</b>", format_duration(model.config.interval) ),
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Box {
                        set_spacing:6,

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "hours"
                            },
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Hours as usize].as_secs_f64() / 3600.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Hours,Duration::from_hours(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "mins"
                            },
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Minutes as usize].as_secs_f64() / 60.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Minutes,Duration::from_mins(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "secs"
                            },
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Seconds as usize].as_secs_f64(),
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Seconds,Duration::from_secs(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "millisecs"
                            },
                            gtk::SpinButton::with_range(1.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Milliseconds as usize].as_millis() as f64,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Milliseconds,Duration::from_millis(spin.value() as u64 )));
                                },
                            }
                        },
                    },
                },

                gtk::Box {
                    set_orientation : gtk::Orientation::Vertical,
                    set_spacing:12,

                    gtk::Label {
                        set_markup: "<b>Settings</b>",
                        set_halign: gtk::Align::Start,
                    },


                    gtk::Box {

                        set_spacing:6,
                        gtk::Label {
                            set_label: "Mode:",
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Box {
                            add_css_class: "linked",
                            #[name = "toggle_btn"]
                            gtk::ToggleButton {
                                set_label: "Hold",
                                set_active: !model.config.toggle,
                                set_tooltip: "Hold hotkey to keep clicking",
                                connect_toggled[sender] =>  move |_| {
                                    sender.input(AppMessages::Toggle(false));
                                }
                            },
                            gtk::ToggleButton {
                                set_label: "Toggle",
                                set_group: Some(&toggle_btn),
                                set_tooltip: "Toggle on/off clicking with hotkey",
                                set_active: model.config.toggle,
                                connect_toggled[sender] =>  move |_| {
                                    sender.input(AppMessages::Toggle(true));
                                }
                            },
                        }

                    },

                    // Hotkey Selector

                    gtk::Box {
                        set_spacing:6,
                        gtk::Label {
                            set_label: "Activation Key:",
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Button {
                            #[watch]
                            set_label: &model.key_label(),
                            connect_clicked[sender] => move |_| {
                                sender.input(AppMessages::CaptureBegin);
                            },
                        },
                    }
                },

            },
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mouse = Device::open(&config.devices.mouse).unwrap();
        let keyboard = Device::open(&config.devices.keyboard).unwrap();
        let mut virtual_mouse = create_virtual_mouse();
        let (click_sender, click_reciever) = mpsc::channel::<()>();
        let mut durations = [Duration::default(); 4];

        let total_secs = config.interval.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let milliseconds = config.interval.subsec_millis() as u64; // the remaining ms part

        durations[AppDuration::Milliseconds as usize] = Duration::from_millis(milliseconds);
        durations[AppDuration::Seconds as usize] = Duration::from_secs(seconds);
        durations[AppDuration::Minutes as usize] = Duration::from_mins(minutes);
        durations[AppDuration::Hours as usize] = Duration::from_hours(hours);

        let duration = durations.iter().sum::<Duration>().max(MIN_DURATION);

        let cx = AppContext {
            toggle: Arc::new(AtomicBool::new(config.toggle)),
            keep_clicking: Arc::new(AtomicBool::new(false)),
            captured_input: Arc::new(AtomicU16::new(config.hotkey.code())),
            capturing: Arc::new(AtomicBool::new(false)),
            sender: sender.clone(),
            duration: Arc::new(AtomicU64::new(duration.as_millis() as u64)),
        };

        let keyboard_context = DeviceContext {
            device: keyboard,
            cx: cx.clone(),
        };

        let mouse_context = DeviceContext {
            device: mouse,
            cx: cx.clone(),
        };

        // keyboard thread
        device_input_handler(keyboard_context);
        // mouse thread
        device_input_handler(mouse_context);

        let t_keep_clicking = cx.keep_clicking.clone();
        let t_duration = cx.duration.clone();
        let t_sender = sender.clone();

        // clicking thread
        spawn(move || {
            loop {
                match click_reciever.recv() {
                    Ok(_) => {
                        while t_keep_clicking.load(SeqCst) {
                            send_left_click(&mut virtual_mouse);
                            let milliseconds = t_duration.load(SeqCst);
                            let duration: Duration =
                                Duration::from_millis(milliseconds).max(MIN_DURATION);
                            sleep(duration);
                        }
                        t_sender.input(AppMessages::ClickingEnd);
                    }
                    Err(_) => {
                        return;
                    }
                }
            }
        });

        let model = Self {
            durations,
            cx,
            click_sender,
            config,
            capturing: false,
            is_clicking: false,
            dirty: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _: relm4::ComponentSender<Self>) {
        match message {
            AppMessages::CaptureBegin => {
                println!("Capturing");
                self.capturing = true;
                self.cx.capturing.store(true, SeqCst);
            }
            AppMessages::Toggle(value) => {
                self.cx.toggle.store(value, SeqCst);
                self.config.toggle = value;
                println!("Toggle set to {}", value);
                self.dirty = true;
                self.save_config();
            }

            AppMessages::CaptureEnd(key) => {
                println!("Captured {:?}", key);
                self.capturing = false;
                self.config.hotkey = key;
                self.dirty = true;
                self.save_config();
            }

            AppMessages::ClickingBegin => {
                if self.cx.keep_clicking.load(SeqCst) {
                    println!("Begin Clicking");
                    self.is_clicking = true;
                    self.click_sender.send(()).unwrap();
                }
            }

            AppMessages::ClickingEnd => {
                println!("Clicking stopped");
                self.is_clicking = false;
            }

            AppMessages::DurationChanged(app_duration, duration) => {
                self.durations[app_duration as usize] = duration;
                let duration = self.durations.iter().sum::<Duration>().max(MIN_DURATION);
                self.cx.duration.store(duration.as_millis() as u64, SeqCst);
                self.config.interval = duration;
                println!("Set duration to {}", format_duration(duration));
                self.dirty = true;
                self.save_config();
            }
        }
    }
}

fn create_virtual_mouse() -> VirtualDevice {
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::BTN_LEFT);

    VirtualDevice::builder()
        .unwrap()
        .name("virtual_mouse")
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap()
}

fn send_left_click(device: &mut VirtualDevice) {
    let events = [
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 1), // press
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device.emit(&events).unwrap();
    let events = [
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 0), // SeqCst
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device.emit(&events).unwrap();
}

fn main() {
    let path = config_path();
    let contents = std::fs::read_to_string(path).unwrap_or_default();
    let config: Config = toml::from_str(&contents).unwrap();
    let app = RelmApp::new("io.github.dev-michaelr.autoclicker");
    app.run::<AppModel>(config);
}
