//! Runs the same `App` as the Android build, on the desktop, for UI checks.
//!
//! cargo run -p mentalcalc-android --example desktop -- \
//!     [--data-dir DIR] [--screen main|session-settings|session|result|archive|settings] \
//!     [--screenshot OUT.png]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui;
use mentalcalc_android::{App, Screen, Storage};

const USAGE: &str = "usage: desktop [--data-dir DIR] [--screen NAME] [--screenshot OUT.png]";
/// A few frames let the layout settle before the capture.
const SCREENSHOT_FRAME: u32 = 6;
/// Portrait phone proportions.
const WINDOW_SIZE: [f32; 2] = [390.0, 844.0];

struct Options {
    data_dir: Option<PathBuf>,
    screen: Screen,
    screenshot: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        data_dir: None,
        screen: Screen::Main,
        screenshot: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => options.data_dir = Some(PathBuf::from(value(&mut args, &arg)?)),
            "--screen" => {
                let name = value(&mut args, &arg)?;
                options.screen =
                    Screen::from_arg(&name).ok_or_else(|| format!("unknown screen: {name}"))?;
            }
            "--screenshot" => options.screenshot = Some(PathBuf::from(value(&mut args, &arg)?)),
            "-h" | "--help" => return Err(USAGE.to_owned()),
            other => return Err(format!("unknown argument: {other}\n{USAGE}")),
        }
    }
    Ok(options)
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn default_data_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("mentalcalc-desktop-{}-{nanos}", std::process::id()))
}

type Shot = Arc<Mutex<Option<Result<PathBuf, String>>>>;

struct Desktop {
    app: App,
    frames: u32,
    requested: bool,
    screenshot: Option<PathBuf>,
    shot: Shot,
}

impl eframe::App for Desktop {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        eframe::App::logic(&mut self.app, ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        eframe::App::ui(&mut self.app, ui, frame);
        let Some(path) = self.screenshot.clone() else {
            return;
        };
        self.frames += 1;
        if !self.requested && self.frames >= SCREENSHOT_FRAME {
            self.requested = true;
            let shot = Arc::clone(&self.shot);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(
                    egui::ScreenshotCallback::new(move |image| {
                        let result = save_png(&image, &path);
                        if let Ok(mut slot) = shot.lock() {
                            *slot = Some(result);
                        }
                    }),
                ));
        }
        let done = self.shot.lock().map(|slot| slot.is_some()).unwrap_or(true);
        if done {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ui.ctx().request_repaint();
        }
    }
}

fn save_png(image: &egui::ColorImage, path: &Path) -> Result<PathBuf, String> {
    let [width, height] = image.size;
    let buffer = image::RgbaImage::from_raw(width as u32, height as u32, image.as_raw().to_vec())
        .ok_or_else(|| "screenshot buffer has the wrong size".to_owned())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    buffer.save(path).map_err(|err| err.to_string())?;
    Ok(path.to_path_buf())
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let storage = Storage::new(options.data_dir.unwrap_or_else(default_data_dir));
    let shot: Shot = Arc::default();
    let screen = options.screen;
    let screenshot = options.screenshot;
    let shot_for_app = Arc::clone(&shot);

    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_title("Mental Calculator"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Mental Calculator",
        native,
        Box::new(move |cc| {
            Ok(Box::new(Desktop {
                app: App::new(cc, storage, screen),
                frames: 0,
                requested: false,
                screenshot,
                shot: shot_for_app,
            }))
        }),
    );
    if let Err(err) = result {
        eprintln!("failed to run window: {err}");
        return ExitCode::FAILURE;
    }
    let outcome = shot.lock().ok().and_then(|mut slot| slot.take());
    match outcome {
        Some(Ok(path)) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Some(Err(message)) => {
            eprintln!("screenshot failed: {message}");
            ExitCode::FAILURE
        }
        None => ExitCode::SUCCESS,
    }
}
