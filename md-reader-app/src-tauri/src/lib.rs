mod file_assoc;
mod file_reader;
mod window_state;

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Theme, Window, WindowEvent};

use file_reader::OpenedFile;

const MAIN_WINDOW: &str = "main";
const OPEN_FILE_EVENT: &str = "open-file";
const WINDOW_STATE_SAVE_INTERVAL: Duration = Duration::from_millis(800);

struct AppState {
    /// File opened from the command line (or a macOS "open with" event that
    /// arrived before the frontend was ready).
    pending_file: Mutex<Option<OpenedFile>>,
    window_state_ready: AtomicBool,
    last_window_save: Mutex<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pending_file: Mutex::new(None),
            window_state_ready: AtomicBool::new(false),
            last_window_save: Mutex::new(Instant::now()),
        }
    }
}

#[tauri::command]
fn open_file(app: AppHandle, path: String) -> Result<OpenedFile, String> {
    open_and_publish(&app, Path::new(&path))
}

#[tauri::command]
fn initial_file(app: AppHandle, state: State<'_, AppState>) -> Option<OpenedFile> {
    let file = state.pending_file.lock().ok()?.take()?;
    set_window_title(&app, &file.name);
    Some(file)
}

fn open_and_publish(app: &AppHandle, path: &Path) -> Result<OpenedFile, String> {
    let loaded = file_reader::read_markdown(path)?;

    // The frontend may only load images from the directory of a file the user
    // explicitly opened. Scope entries accumulate for the session, which is
    // fine: every entry comes from an explicit open action.
    if let Err(err) = app
        .asset_protocol_scope()
        .allow_directory(&loaded.canonical_base_dir, true)
    {
        eprintln!("failed to allow asset directory: {err}");
    }

    set_window_title(app, &loaded.file.name);
    Ok(loaded.file)
}

fn set_window_title(app: &AppHandle, name: &str) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.set_title(name);
    }
}

fn sync_window_theme(window: &Window) {
    let color = match window.theme() {
        Ok(Theme::Dark) => Color(27, 27, 29, 255),
        _ => Color(255, 255, 255, 255),
    };
    let _ = window.set_background_color(Some(color));
}

fn save_window_state(window: &Window) {
    let state = window.state::<AppState>();
    if !state.window_state_ready.load(Ordering::SeqCst) {
        return;
    }
    if let Err(err) = window_state::save(window) {
        eprintln!("failed to save window state: {err}");
    }
}

fn save_window_state_throttled(window: &Window) {
    let state = window.state::<AppState>();
    if !state.window_state_ready.load(Ordering::SeqCst) {
        return;
    }

    let should_save = state
        .last_window_save
        .lock()
        .map(|mut last| {
            if last.elapsed() >= WINDOW_STATE_SAVE_INTERVAL {
                *last = Instant::now();
                true
            } else {
                false
            }
        })
        .unwrap_or(false);

    if should_save && let Err(err) = window_state::save(window) {
        eprintln!("failed to save window state: {err}");
    }
}

#[cfg(target_os = "macos")]
fn handle_opened_urls(app: &AppHandle, urls: Vec<tauri::Url>) {
    let Some(path) = file_assoc::path_from_urls(&urls) else {
        return;
    };

    match open_and_publish(app, &path) {
        Ok(file) => {
            let state = app.state::<AppState>();
            if let Ok(mut pending) = state.pending_file.lock() {
                *pending = Some(file.clone());
            }
            if let Err(err) = app.emit(OPEN_FILE_EVENT, file) {
                eprintln!("failed to deliver opened file: {err}");
            }
        }
        Err(err) => eprintln!("failed to open file from system: {err}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![open_file, initial_file])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();

            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                let window = window.as_ref().window();
                sync_window_theme(&window);

                if let Some(saved) = window_state::load(&handle) {
                    window_state::restore(&window, &saved);
                }

                if let Some(path) = file_assoc::path_from_args(std::env::args_os()) {
                    match open_and_publish(&handle, &path) {
                        Ok(file) => {
                            if let Ok(mut pending) = state.pending_file.lock() {
                                *pending = Some(file);
                            }
                        }
                        Err(err) => eprintln!("failed to open initial file: {err}"),
                    }
                }

                window.show()?;
                window.set_focus()?;
            }

            state.window_state_ready.store(true, Ordering::SeqCst);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::ThemeChanged(_) => sync_window_theme(window),
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => save_window_state_throttled(window),
            WindowEvent::CloseRequested { .. } => save_window_state(window),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => handle_opened_urls(app_handle, urls),
        RunEvent::ExitRequested { .. } => {
            if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW) {
                save_window_state(&window.as_ref().window());
            }
        }
        _ => {}
    });
}
