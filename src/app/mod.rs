mod input;
pub mod save;

use anyhow::Context;
use evdev::{Device, KeyCode};
use gtk::prelude::*;
use humantime::format_duration;
use input::*;

use relm4::*;
use save::{Config, MIN_DURATION};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64};
use std::sync::mpsc;
use std::thread::{sleep, spawn};
use std::time::Duration;

#[derive(Debug)]
pub enum AppDuration {
    Milliseconds = 0,
    Seconds = 1,
    Minutes = 2,
    Hours = 3,
}

pub struct AppModel {
    durations: [Duration; 4],
    capturing: bool,
    is_clicking: bool,
    config: Config,
    click_sender: mpsc::Sender<()>,
    cx: &'static AppContext,
    dirty: bool,
}

pub struct AppContext {
    toggle: AtomicBool,
    keep_clicking: AtomicBool,
    captured_input: AtomicU16,
    capturing: AtomicBool,
    duration: AtomicU64,
    sender: ComponentSender<AppModel>,
}

#[derive(Debug)]
pub enum AppMessages {
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
}

fn exit_on_err<T>(r: anyhow::Result<T>) -> T {
    r.unwrap_or_else(|e| {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    })
}

#[relm4::component(pub)]
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
        let mouse =
            exit_on_err(Device::open(&config.devices.mouse).context("Failed to open mouse device"));

        let keyboard = exit_on_err(
            Device::open(&config.devices.keyboard).context("Failed to open keyboard device"),
        );

        let mut virtual_mouse = exit_on_err(create_virtual_mouse());

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

        let cx: &'static AppContext = Box::leak(Box::new(AppContext {
            toggle: AtomicBool::new(config.toggle),
            keep_clicking: AtomicBool::new(false),
            captured_input: AtomicU16::new(config.hotkey.code()),
            capturing: AtomicBool::new(false),
            sender: sender.clone(),
            duration: AtomicU64::new(duration.as_millis() as u64),
        }));

        // keyboard thread
        device_input_handler(keyboard, cx);
        // mouse thread
        device_input_handler(mouse, cx);

        let t_sender = sender.clone();

        // clicking thread
        spawn(move || {
            loop {
                match click_reciever.recv() {
                    Ok(_) => {
                        while cx.keep_clicking.load(SeqCst) {
                            send_left_click(&mut virtual_mouse).unwrap_or_else(|e| {
                                eprintln!("Error: {e:#}");
                                std::process::exit(1);
                            });
                            let milliseconds = cx.duration.load(SeqCst);
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
                exit_on_err(self.save_config());
            }

            AppMessages::CaptureEnd(key) => {
                println!("Captured {:?}", key);
                self.capturing = false;
                self.config.hotkey = key;
                self.dirty = true;
                exit_on_err(self.save_config());
            }

            AppMessages::ClickingBegin => {
                if self.cx.keep_clicking.load(SeqCst) {
                    println!("Begin Clicking");
                    self.is_clicking = true;
                    exit_on_err(
                        self.click_sender
                            .send(())
                            .context("Failed to send signal through channel"),
                    );
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
                exit_on_err(self.save_config());
            }
        }
    }
}
