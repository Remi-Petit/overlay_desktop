// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use std::sync::Mutex;

struct GstProcess(Mutex<Option<std::process::Child>>);

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resolve the GStreamer base directory (contains bin/ and lib/).
fn gstreamer_base_dir() -> Result<std::path::PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot get exe parent dir")?
        .to_path_buf();
    let base = exe_dir.join("binaries").join("gstreamer_1.28");
    if base.join("bin").exists() {
        return Ok(base);
    }
    // Dev fallback: look relative to the src-tauri folder
    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("gstreamer_1.28");
    if dev.join("bin").exists() {
        return Ok(dev);
    }
    Err(format!(
        "GStreamer dir not found. Tried:\n  {}\n  {}",
        base.display(),
        dev.display()
    ))
}

/// Build a Command for gst-launch with proper env (PATH + GST_PLUGIN_PATH).
fn gst_command() -> Result<std::process::Command, String> {
    let base = gstreamer_base_dir()?;
    let bin_dir = base.join("bin");
    let plugin_dir = base.join("lib").join("gstreamer-1.0");
    let exe_path = bin_dir.join("gst-launch-1.0-x86_64-pc-windows-msvc.exe");

    if !exe_path.exists() {
        return Err(format!("Binary not found at: {}", exe_path.display()));
    }

    let mut cmd = std::process::Command::new(exe_path);
    cmd.current_dir(&bin_dir)
        .env("PATH", format!("{};{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()))
        .env("GST_PLUGIN_PATH", plugin_dir.display().to_string())
        .env("GST_PLUGIN_SYSTEM_PATH", "");
    Ok(cmd)
}

#[tauri::command]
async fn test_gstreamer() -> Result<String, String> {
    let mut cmd = gst_command()?;
    let output = cmd
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute GStreamer: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "GStreamer error: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[tauri::command]
fn start_stream(state: tauri::State<'_, GstProcess>) -> Result<String, String> {
    let mut process = state.0.lock().map_err(|e| e.to_string())?;
    if process.is_some() {
        return Err("Stream is already running".to_string());
    }

    let mut cmd = gst_command()?;
    let child = cmd
        .args([
            "d3d11screencapturesrc",
            "!", "video/x-raw,framerate=30/1",
            "!", "queue",
            "!", "videoconvert",
            "!", "autovideosink",
        ])
        .spawn()
        .map_err(|e| format!("Failed to start stream: {}", e))?;

    let pid = child.id();
    *process = Some(child);
    Ok(format!("Stream started (PID: {})", pid))
}

#[tauri::command]
fn stop_stream(state: tauri::State<'_, GstProcess>) -> Result<String, String> {
    let mut process = state.0.lock().map_err(|e| e.to_string())?;
    match process.take() {
        Some(mut child) => {
            child.kill().map_err(|e| format!("Failed to kill stream: {}", e))?;
            child.wait().ok();
            Ok("Stream stopped".to_string())
        }
        None => Err("No stream is running".to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(GstProcess(Mutex::new(None)))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_ghost_mode, test_gstreamer, start_stream, stop_stream])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() == "main" {
                    std::process::exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
