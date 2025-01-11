//! This example shows that you can use egui in parallel from multiple threads.

use std::default;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use eframe::egui::menu::BarState;
use eframe::wgpu::Backends;
use eframe::{egui, wgpu};
// use eframe::wgpu::Backends;

const APP_NAME: &str = "fastsm";

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        // renderer: Renderer::default(),
        // run_and_return: true,
        // event_loop_builder: None,
        // window_builder: None,
        // centered: false,
        // wgpu_options: egui_wgpu::WgpuConfiguration::default(),
        // persist_window: true,
        // persistence_path: None,
        // dithering: true,
        viewport: egui::ViewportBuilder::default().with_inner_size([1600.0, 900.0]), // with_inner_size([1024.0, 768.0])
        vsync: true,
        dithering: false,
        hardware_acceleration: eframe::HardwareAcceleration::Required,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Immediate,
            desired_maximum_frame_latency: Some(2),
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew {
                supported_backends: Backends::PRIMARY,
                power_preference: eframe::wgpu::PowerPreference::LowPower,
                device_descriptor: Arc::new(|_| {
                    wgpu::DeviceDescriptor {
                        label: Some("fastsm egui wgpu device"),
                        required_features: wgpu::Features::default(),
                        required_limits: wgpu::Limits {
                            // When using a depth buffer, we have to be able to create a texture
                            // large enough for the entire surface, and we want to support 4k+ displays.
                            max_texture_dimension_2d: 8192,
                            ..wgpu::Limits::default()
                        },
                        memory_hints: wgpu::MemoryHints::Performance,
                    }
                }),
            },
            on_surface_error: Arc::new(|_| eframe::egui_wgpu::SurfaceErrorAction::SkipFrame),
        },
        window_builder: Some(Box::new(|builder| {
            // let icon = eframe::icon_data::from_png_bytes(include_bytes!("../images/icon.png"); ).unwrap(),
            builder
                .with_resizable(true)
                .with_title(APP_NAME)
                .with_app_id(APP_NAME)
        })),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

/// State per thread.
struct ThreadState {
    thread_nr: usize,
    title: String,
    name: String,
    age: u32,
}

impl ThreadState {
    fn new(thread_nr: usize) -> Self {
        let title = format!("Background thread {thread_nr}");
        Self {
            thread_nr,
            title,
            name: "Arthur".into(),
            age: 12 + thread_nr as u32 * 10,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        let pos = egui::pos2(16.0, 128.0 * (self.thread_nr as f32 + 1.0));
        egui::Window::new(&self.title)
            .default_pos(pos)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Your name: ");
                    ui.text_edit_singleline(&mut self.name);
                });
                ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
                if ui.button("Increment").clicked() {
                    self.age += 1;
                }
                ui.label(format!("Hello '{}', age {}", self.name, self.age));
            });
    }
}

fn new_worker(
    thread_nr: usize,
    on_done_tx: mpsc::SyncSender<()>,
) -> (JoinHandle<()>, mpsc::SyncSender<egui::Context>) {
    let (show_tx, show_rc) = mpsc::sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("EguiPanelWorker {thread_nr}"))
        .spawn(move || {
            let mut state = ThreadState::new(thread_nr);
            while let Ok(ctx) = show_rc.recv() {
                state.show(&ctx);
                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

struct MyApp {
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
}

impl MyApp {
    fn new() -> Self {
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);

        let mut slf = Self {
            threads,
            on_done_tx,
            on_done_rc,
        };

        slf.spawn_thread();
        slf.spawn_thread();

        slf
    }

    fn spawn_thread(&mut self) {
        let thread_nr = self.threads.len();
        self.threads
            .push(new_worker(thread_nr, self.on_done_tx.clone()));
    }
}

impl std::ops::Drop for MyApp {
    fn drop(&mut self) {
        for (handle, show_tx) in self.threads.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Main thread").show(ctx, |ui| {
            if ui.button("Spawn another thread").clicked() {
                self.spawn_thread();
            }
        });

        for (_handle, show_tx) in &self.threads {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.threads.len() {
            let _ = self.on_done_rc.recv();
        }
    }
}
