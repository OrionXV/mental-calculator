//! Mental arithmetic trainer UI on egui. The same `App` runs in NativeActivity on
//! Android and in the desktop example used for screenshots.

pub mod app;
pub mod state;
pub mod storage;
pub mod strings;

pub use app::App;
pub use state::{Action, AppState, Screen};
pub use storage::Storage;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: android_activity::AndroidApp) {
    use std::path::PathBuf;

    // Without a logger, `log` output is dropped. Errors then reach logcat (`adb logcat -s mentalcalc`).
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("mentalcalc"),
    );
    let data_dir = android_app
        .internal_data_path()
        .unwrap_or_else(|| PathBuf::from("."));
    let storage = Storage::new(data_dir);
    let mut options = eframe::NativeOptions::default();
    options.android_app = Some(android_app);
    let result = eframe::run_native(
        "Mental Calculator",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, storage, Screen::Main)))),
    );
    if let Err(err) = result {
        log::error!("failed to start: {err}");
    }
}
