// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resolve the GStreamer sidecar binary path.
/// In dev mode the exe sits under src-tauri/target/debug,
/// in production it sits next to the bundled app.
fn gstreamer_bin_dir() -> Result<std::path::PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot get exe parent dir")?
        .to_path_buf();
    let bin_dir = exe_dir.join("binaries").join("gstreamer_1.28").join("bin");
    if bin_dir.exists() {
        return Ok(bin_dir);
    }
    // Dev fallback: look relative to the src-tauri folder
    let dev_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("gstreamer_1.28")
        .join("bin");
    if dev_dir.exists() {
        return Ok(dev_dir);
    }
    Err(format!(
        "GStreamer bin dir not found. Tried:\n  {}\n  {}",
        bin_dir.display(),
        dev_dir.display()
    ))
}

#[tauri::command]
async fn test_gstreamer() -> Result<String, String> {
    let bin_dir = gstreamer_bin_dir()?;
    let exe_path = bin_dir.join("gst-launch-1.0-x86_64-pc-windows-msvc.exe");

    if !exe_path.exists() {
        return Err(format!("Binary not found at: {}", exe_path.display()));
    }

    let output = std::process::Command::new(&exe_path)
        .arg("--version")
        // Set the working dir to the bin folder so DLLs are found
        .current_dir(&bin_dir)
        // Also add the bin dir to PATH for DLL resolution
        .env("PATH", format!("{};{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()))
        .output()
        .map_err(|e| format!("Failed to execute GStreamer: {} (path: {})", e, exe_path.display()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "GStreamer exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_ghost_mode, test_gstreamer])
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
