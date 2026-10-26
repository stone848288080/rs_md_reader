use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Window};

const STATE_FILE: &str = "window-state.json";
const MIN_WIDTH: f64 = 480.0;
const MIN_HEIGHT: f64 = 360.0;
const MIN_VISIBLE_WIDTH: f64 = 120.0;
const MIN_VISIBLE_HEIGHT: f64 = 80.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SavedWindowState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(STATE_FILE))
}

pub fn load(app: &AppHandle) -> Option<SavedWindowState> {
    let data = fs::read_to_string(state_path(app)?).ok()?;
    let state: SavedWindowState = serde_json::from_str(&data).ok()?;

    let is_sane = state.x.is_finite()
        && state.y.is_finite()
        && state.width.is_finite()
        && state.height.is_finite()
        && state.width >= MIN_WIDTH
        && state.height >= MIN_HEIGHT;

    is_sane.then_some(state)
}

pub fn restore(window: &Window, state: &SavedWindowState) {
    let _ = window.set_size(LogicalSize::new(state.width, state.height));
    if is_on_screen(window, state) {
        let _ = window.set_position(LogicalPosition::new(state.x, state.y));
    }
}

/// Avoid restoring a window onto a monitor that no longer exists.
fn is_on_screen(window: &Window, state: &SavedWindowState) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }

    monitors.iter().any(|monitor| {
        let scale = monitor.scale_factor();
        if scale <= 0.0 {
            return false;
        }

        let position = monitor.position();
        let size = monitor.size();
        let left = position.x as f64 / scale;
        let top = position.y as f64 / scale;
        let right = left + size.width as f64 / scale;
        let bottom = top + size.height as f64 / scale;

        let overlap_width = (state.x + state.width).min(right) - state.x.max(left);
        let overlap_height = (state.y + state.height).min(bottom) - state.y.max(top);

        overlap_width >= MIN_VISIBLE_WIDTH && overlap_height >= MIN_VISIBLE_HEIGHT
    })
}

pub fn save(window: &Window) -> Result<(), String> {
    let app = window.app_handle();
    let path = state_path(app).ok_or_else(|| "无法确定配置目录".to_string())?;

    let scale = window
        .scale_factor()
        .map_err(|err| format!("无法读取缩放比例：{err}"))?;
    if scale <= 0.0 {
        return Err("缩放比例无效".to_string());
    }

    let position = window
        .outer_position()
        .map_err(|err| format!("无法读取窗口位置：{err}"))?;
    let size = window
        .outer_size()
        .map_err(|err| format!("无法读取窗口尺寸：{err}"))?;

    let state = SavedWindowState {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
        width: (size.width as f64 / scale).max(MIN_WIDTH),
        height: (size.height as f64 / scale).max(MIN_HEIGHT),
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建配置目录：{err}"))?;
    }

    let data =
        serde_json::to_string_pretty(&state).map_err(|err| format!("无法序列化窗口状态：{err}"))?;
    fs::write(&path, data).map_err(|err| format!("无法写入窗口状态：{err}"))
}
