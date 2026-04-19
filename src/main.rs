use evdev::uinput::VirtualDevice;
use evdev::*;
const MOUSE: &str = "/dev/input/by-id/usb-Razer_Razer_DeathAdder_V2-event-mouse";
const KEYBOARD: &str = "/dev/input/by-id/usb-Corsair_CORSAIR_K70_RGB_PRO_Mechanical_Gaming_Keyboard_5A26F8981EBE3651A45E0500D0491782-event-kbd";
use gtk::prelude::*;
use humantime::format_duration;
use relm4::*;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64};
use std::sync::{Arc, mpsc};
use std::thread::{sleep, spawn};
use std::time::Duration;

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
    click_sender: mpsc::Sender<()>,
    cx: AppContext,
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
            false => format!("{:?}", KeyCode::new(self.cx.captured_input.load(SeqCst))),
        }
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
                        if cx.toggle.load(SeqCst) == true {
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
                        if cx.toggle.load(SeqCst) == true {
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
    type Init = ();
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
                                set_value: model.durations[AppDuration::Minutes as usize].as_secs_f64() * 60.0,
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
                            gtk::SpinButton::with_range(0.0,10000.0,1.0) {
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
                                sender.input(AppMessages::CaptureBegin);
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
        let (click_sender, click_reciever) = mpsc::channel::<()>();
        let mut durations = [Duration::default(); 4];

        // assign some defaults
        durations[AppDuration::Milliseconds as usize] = Duration::from_millis(1);
        durations[AppDuration::Seconds as usize] = Duration::from_millis(0);
        durations[AppDuration::Minutes as usize] = Duration::from_millis(0);
        durations[AppDuration::Hours as usize] = Duration::from_millis(0);

        let cx = AppContext {
            toggle: Arc::new(AtomicBool::new(false)),
            keep_clicking: Arc::new(AtomicBool::new(false)),
            captured_input: Arc::new(AtomicU16::new(KeyCode::BTN_EXTRA.code())),
            capturing: Arc::new(AtomicBool::new(false)),
            sender: sender.clone(),
            duration: Arc::new(AtomicU64::new(
                durations.iter().sum::<Duration>().as_millis() as u64,
            )),
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
        let min_duration = Duration::from_nanos(500);
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
                                Duration::from_millis(milliseconds).max(min_duration);
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
            capturing: false,
            is_clicking: false,
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
            }

            AppMessages::CaptureEnd(key) => {
                println!("Captured {:?}", key);
                self.capturing = false;
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
                let duration: Duration = self.durations.iter().sum();
                self.cx.duration.store(duration.as_millis() as u64, SeqCst);
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
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.code(), 0), // SeqCst
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ];
    device.emit(&events).unwrap();
}

fn main() {
    let app = RelmApp::new("relm4.test.simple");
    app.run::<AppModel>(());
}
