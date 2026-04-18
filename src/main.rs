use evdev::uinput::VirtualDevice;
use evdev::*;
const MOUSE: &str = "/dev/input/by-id/usb-Razer_Razer_DeathAdder_V2-event-mouse";
const KEYBOARD: &str = "/dev/input/by-id/usb-Corsair_CORSAIR_K70_RGB_PRO_Mechanical_Gaming_Keyboard_5A26F8981EBE3651A45E0500D0491782-event-kbd";
use gtk::prelude::*;
use humantime::format_duration;
use relm4::*;
use std::sync::Arc;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64};
use std::thread::{sleep, spawn};
use std::time::Duration;

enum AppDuration {
    Milliseconds = 0,
    Seconds = 1,
    Minutes = 2,
    Hours = 3,
}

struct AppModel {
    durations: [Duration; 4],
    duration: Arc<AtomicU64>,
    capturing: bool,
    toggle: Arc<AtomicBool>,
    is_clicking: Arc<AtomicBool>,
    captured_input: Arc<AtomicU16>,
}

struct DeviceContext {
    device: Device,
    toggle: Arc<AtomicBool>,
    is_clicking: Arc<AtomicBool>,
    sender: ComponentSender<AppModel>,
    captured_input: Arc<AtomicU16>,
}

#[derive(Debug)]
enum AppMessages {
    StartCapturing,
    InputCaptured(KeyCode),
    Toggle(bool),
    DurationChanged(usize, Duration),
}

impl AppModel {
    fn key_label(&self) -> String {
        match self.capturing {
            true => "Press any key...".to_string(),
            false => format!("{:?}", KeyCode::new(self.captured_input.load(Acquire))),
        }
    }
}

fn device_input_handler(mut cx: DeviceContext) {
    spawn(move || {
        loop {
            let device_events = cx.device.fetch_events().unwrap();
            for event in device_events {
                match event.destructure() {
                    // key down
                    EventSummary::Key(_, key, 1) => {
                        cx.sender.input(AppMessages::InputCaptured(key));

                        if key.code() != cx.captured_input.load(Acquire) {
                            continue;
                        }

                        if cx.toggle.load(Acquire) == true {
                            // toggle is enabled so we flip
                            cx.is_clicking.fetch_not(AcqRel);
                            continue;
                        }

                        cx.is_clicking.store(true, Release);
                    }

                    // key up
                    EventSummary::Key(_, key, 0) => {
                        if key.code() != cx.captured_input.load(Acquire) {
                            continue;
                        }

                        if cx.toggle.load(Acquire) == true {
                            // toggle is enabled so we dont turn off
                            continue;
                        }

                        cx.is_clicking.store(false, Release);
                    }
                    _ => (),
                };
            }
        }
    });
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMessages;
    type Output = ();

    view! {
        gtk::Window {
            set_default_size: (100,100),

            #[watch]
            set_can_target: !( model.capturing || model.is_clicking.load(Acquire) ),

            gtk::Box {
                set_orientation : gtk::Orientation::Vertical,
                set_margin_all:6,
                set_spacing:12,
                gtk::Box {
                    set_orientation : gtk::Orientation::Vertical,
                    set_spacing:12,
                    gtk::Label {
                        #[watch]
                        set_markup: &format!("<b>Click Interval: {}</b>",  format_duration(model.durations.iter().sum::<Duration>()) ),
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
                                set_value: model.durations[AppDuration::Hours as usize].as_secs_f64() * 60.0 * 60.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Hours as usize,Duration::from_hours(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "mins"
                            },
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Minutes as usize].as_secs_f64() * 60.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Minutes as usize,Duration::from_mins(spin.value() as u64 )));
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
                                    sender.input(AppMessages::DurationChanged(AppDuration::Seconds as usize,Duration::from_secs(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "millisecs"
                            },
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
                                set_value: model.durations[AppDuration::Milliseconds as usize].as_millis() as f64,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged(AppDuration::Milliseconds as usize,Duration::from_millis(spin.value() as u64 )));
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
                                set_active: true,
                                set_tooltip: "Hold hotkey to keep clicking",
                                connect_toggled[sender] =>  move |_| {
                                    sender.input(AppMessages::Toggle(false));
                                }
                            },
                            gtk::ToggleButton {
                                set_label: "Toggle",
                                set_group: Some(&toggle_btn),
                                set_tooltip: "Toggle on/off clicking with hotkey",
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
                                sender.input(AppMessages::StartCapturing);
                            },
                        },
                    }
                },


            },

        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mouse = Device::open(MOUSE).unwrap();
        let keyboard = Device::open(KEYBOARD).unwrap();
        let mut virtual_mouse = create_virtual_mouse();
        let captured_input = Arc::new(AtomicU16::new(KeyCode::BTN_EXTRA.code()));
        let toggle = Arc::new(AtomicBool::new(false));
        let mut durations = [Duration::default(); 4];

        // assign some defaults
        durations[AppDuration::Milliseconds as usize] = Duration::from_millis(500);
        durations[AppDuration::Seconds as usize] = Duration::from_millis(0);
        durations[AppDuration::Minutes as usize] = Duration::from_millis(0);
        durations[AppDuration::Hours as usize] = Duration::from_millis(0);

        let sum: Duration = durations.iter().sum();

        let duration = Arc::new(AtomicU64::new(sum.as_millis() as u64));
        let is_clicking = Arc::new(AtomicBool::new(false));

        let keyboard_context = DeviceContext {
            device: keyboard,
            toggle: toggle.clone(),
            is_clicking: is_clicking.clone(),
            sender: sender.clone(),
            captured_input: captured_input.clone(),
        };

        let mouse_context = DeviceContext {
            device: mouse,
            toggle: toggle.clone(),
            is_clicking: is_clicking.clone(),
            sender: sender.clone(),
            captured_input: captured_input.clone(),
        };

        // keyboard thread
        device_input_handler(keyboard_context);
        // mouse thread
        device_input_handler(mouse_context);

        let t_is_clicking = is_clicking.clone();
        let t_duration = duration.clone();
        let min_duration = Duration::from_nanos(500);

        // clicking thread
        spawn(move || {
            loop {
                if t_is_clicking.load(Acquire) {
                    send_left_click(&mut virtual_mouse);
                }
                let milliseconds = t_duration.load(Acquire);
                let duration: Duration = Duration::from_millis(milliseconds).max(min_duration);
                sleep(duration);
            }
        });

        let model = Self {
            duration,
            durations,
            capturing: false,
            captured_input,
            is_clicking,
            toggle,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _: relm4::ComponentSender<Self>) {
        match message {
            AppMessages::StartCapturing => {
                println!("Begin Capture");
                self.capturing = true;
            }
            AppMessages::InputCaptured(input) => {
                if self.capturing {
                    self.capturing = false;
                    println!("Captured {:?}", input);
                    self.captured_input.store(input.code(), Release);
                }
            }
            AppMessages::Toggle(value) => {
                self.toggle.store(value, Release);
            }

            AppMessages::DurationChanged(index, duration) => {
                self.durations[index] = duration;
                let duration: Duration = self.durations.iter().sum();
                self.duration.store(duration.as_millis() as u64, Release);

                println!("Set duration to {:?}", duration);
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
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 0), // release
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device.emit(&events).unwrap();
}

fn main() {
    println!("Hello, world!");
    let app = RelmApp::new("relm4.test.simple");
    app.run::<AppModel>(());
}
