//! Mental arithmetic trainer UI on egui. The same `App` runs in NativeActivity on
//! Android and in the desktop example used for screenshots.

pub mod app;
pub mod safe_area;
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
    let options = eframe::NativeOptions {
        android_app: Some(android_app.clone()),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Mental Calculator",
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, storage, Screen::Main);
            app.set_android_app(android_app);
            Ok(Box::new(app))
        }),
    );
    if let Err(err) = result {
        log::error!("failed to start: {err}");
    }
    // winit creates its event loop once per process and cannot create a second one.
    // A later launch in the same process would then hang on the splash screen, so end
    // the process whenever the activity returns and let the next launch start clean.
    std::process::exit(0);
}
