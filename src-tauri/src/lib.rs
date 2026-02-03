// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{AppHandle, Manager, WebviewWindow, ipc::Response};
use xcap::Monitor;
use image::imageops;

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn capture_overlay(app: AppHandle, window: WebviewWindow, target_label: Option<String>) -> Result<Response, String> {
    
    // 1. Logique de ciblage
    let target_window = if let Some(label) = target_label {
        app.get_webview_window(&label).ok_or(format!("Fenêtre '{}' introuvable", label))?
    } else {
        window
    };
    
    let window_pos = target_window.outer_position().map_err(|e| e.to_string())?;
    let window_size = target_window.outer_size().map_err(|e| e.to_string())?;

    // 2. Thread de capture
    // CORRECTION : On précise le type de retour -> Result<Vec<u8>, String>
    let binary_data = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, String> {
        
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        
        let monitor = monitors.into_iter().find(|m| {
             let x = m.x().unwrap_or(0);
             let y = m.y().unwrap_or(0);
             let w = m.width().unwrap_or(0);
             let h = m.height().unwrap_or(0);
             window_pos.x >= x && window_pos.x < x + w as i32 &&
             window_pos.y >= y && window_pos.y < y + h as i32
        })
        // CORRECTION : On convertit le message d'erreur statique en String
        .ok_or("Moniteur introuvable".to_string())?;

        let image = monitor.capture_image().map_err(|e| e.to_string())?;
        
        let m_x = monitor.x().map_err(|e| e.to_string())?;
        let m_y = monitor.y().map_err(|e| e.to_string())?;
        
        let crop_x = (window_pos.x - m_x).max(0) as u32;
        let crop_y = (window_pos.y - m_y).max(0) as u32;
        let crop_w = window_size.width.min(image.width() - crop_x);
        let crop_h = window_size.height.min(image.height() - crop_y);

        let cropped_image = imageops::crop_imm(&image, crop_x, crop_y, crop_w, crop_h).to_image();
        
        let width = cropped_image.width();
        let height = cropped_image.height();
        let mut pixels = cropped_image.into_raw();

        // 3. Construction du paquet binaire
        let mut response_buffer = Vec::with_capacity(8 + pixels.len());
        
        response_buffer.extend_from_slice(&width.to_le_bytes());
        response_buffer.extend_from_slice(&height.to_le_bytes());
        response_buffer.append(&mut pixels);

        Ok(response_buffer)

    }).await.map_err(|e| e.to_string())??; 
    // Le double ?? gère l'erreur du thread ET l'erreur interne (String)

    Ok(Response::new(binary_data))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_ghost_mode, capture_overlay])
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
