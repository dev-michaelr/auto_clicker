use evdev::uinput::VirtualDevice;
use evdev::*;
const MOUSE: &str = "/dev/input/by-id/usb-Razer_Razer_DeathAdder_V2-event-mouse";
const KEYBOARD: &str = "/dev/input/by-id/usb-Corsair_CORSAIR_K70_RGB_PRO_Mechanical_Gaming_Keyboard_5A26F8981EBE3651A45E0500D0491782-event-kbd";
use gtk::prelude::*;
use relm4::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct AppModel {
    durations: Arc<Mutex<Vec<Duration>>>,
    capturing: bool,
    toggle: Arc<AtomicBool>,
    captured_input: Arc<Mutex<KeyCode>>,
}

struct DeviceContext {
    device: Device,
    toggle: Arc<AtomicBool>,
    is_clicking: Arc<AtomicBool>,
    sender: ComponentSender<AppModel>,
    captured_input: Arc<Mutex<KeyCode>>,
}

#[derive(Debug)]
enum DurationType {
    Hours(u64),
    Mins(u64),
    Secs(u64),
    Milli(u64),
}

impl DurationType {
    fn to_index_and_duration(self) -> (usize, Duration) {
        match self {
            Self::Milli(ms) => (0, Duration::from_millis(ms)),
            Self::Secs(s) => (1, Duration::from_secs(s)),
            Self::Mins(m) => (2, Duration::from_secs(m)),
            Self::Hours(h) => (3, Duration::from_hours(h)),
        }
    }
}

#[derive(Debug)]
enum AppMessages {
    StartCapturing,
    InputCaptured(KeyCode),
    Toggle(bool),
    DurationChanged(DurationType),
}

impl AppModel {
    fn key_label(&self) -> String {
        match self.capturing {
            true => "Press any key...".to_string(),
            false => format!("{:?}", self.captured_input.lock().unwrap()),
        }
    }
}

fn device_input_handler(mut cx: DeviceContext) {
    thread::spawn(move || {
        loop {
            let device_events = cx.device.fetch_events().unwrap();

            for event in device_events {
                match event.destructure() {
                    // key down
                    EventSummary::Key(_, key, 1) => {
                        cx.sender.input(AppMessages::InputCaptured(key));

                        if key != *cx.captured_input.lock().unwrap() {
                            continue;
                        }

                        if cx.toggle.load(Ordering::Relaxed) == true {
                            // toggle is enabled so we flip
                            cx.is_clicking.fetch_not(Ordering::Relaxed);
                            continue;
                        }

                        cx.is_clicking.store(true, Ordering::Relaxed);
                    }

                    // key up
                    EventSummary::Key(_, key, 0) => {
                        if key != *cx.captured_input.lock().unwrap() {
                            continue;
                        }

                        if cx.toggle.load(Ordering::Relaxed) == true {
                            // toggle is enabled so we dont turn off
                            continue;
                        }

                        cx.is_clicking.store(false, Ordering::Relaxed);
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

            gtk::Box {
                set_orientation : gtk::Orientation::Vertical,
                set_margin_all:6,
                set_spacing:12,
                gtk::Box {
                    set_orientation : gtk::Orientation::Vertical,
                    set_spacing:12,
                    gtk::Label {
                        set_markup: "<b>Click Interval:</b>",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Box {
                        set_spacing:6,

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "hours"
                            },
                            gtk::SpinButton::with_range(0.0,100.0,1.0) {
                                set_value: 0.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged( DurationType::Hours(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "mins"
                            },
                            gtk::SpinButton::with_range(0.0,100.0,1.0) {
                                set_value: 0.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged( DurationType::Mins(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "secs"
                            },
                            gtk::SpinButton::with_range(0.0,100.0,1.0) {
                                set_value: 0.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged( DurationType::Secs(spin.value() as u64 )));
                                },
                            }
                        },

                        gtk::Box {
                            set_spacing:6,
                            gtk::Label {
                                set_label: "millisecs"
                            },
                            gtk::SpinButton::with_range(0.0,100.0,1.0) {
                                set_value: 10.0,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(AppMessages::DurationChanged( DurationType::Milli(spin.value() as u64 )));
                                },
                            }
                        },
                    }
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

                        set_tooltip: "Makes the hotkey act like a switch",

                        gtk::Label {
                            set_label: "Should toggle:",
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Switch {
                            connect_active_notify[sender] => move |switch| {
                                sender.input(AppMessages::Toggle(switch.is_active()));
                            }
                        },
                    },

                    // Hotkey Selector

                    gtk::Box {
                        set_spacing:6,
                        gtk::Label {
                            set_label: "Hotkey:",
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Button {
                            #[watch]
                            set_label: &model.key_label(),
                            set_focusable: false,
                            connect_clicked[sender] => move |_| {
                                sender.input(AppMessages::StartCapturing);
                            },
                        },
                    }
                }

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

        let captured_input = Arc::new(Mutex::new(KeyCode::KEY_A));
        let toggle = Arc::new(AtomicBool::new(false));
        let durations = Arc::new(Mutex::new(vec![
            Duration::from_millis(10),
            Duration::from_millis(0),
            Duration::from_millis(0),
            Duration::from_millis(0),
        ]));
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
        let t_durations = durations.clone();
        let min_duration = Duration::from_nanos(10);

        // clicking thread
        thread::spawn(move || {
            loop {
                if t_is_clicking.load(Ordering::Relaxed) {
                    send_left_click(&mut virtual_mouse);
                }

                let duration: Duration = t_durations.lock().unwrap().iter().sum();

                thread::sleep(duration.max(min_duration));
            }
        });

        let model = Self {
            durations: durations,
            capturing: false,
            captured_input: captured_input,
            toggle: toggle,
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
                    let mut captured = self.captured_input.lock().unwrap();
                    *captured = input;
                }
            }
            AppMessages::Toggle(value) => {
                self.toggle.store(value, Ordering::Relaxed);
            }

            AppMessages::DurationChanged(duration_type) => {
                let (index, duration) = duration_type.to_index_and_duration();
                self.durations.lock().unwrap()[index] = duration;
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
