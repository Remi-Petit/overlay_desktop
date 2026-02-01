// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use xcap::Monitor;
use image::{DynamicImage, ImageFormat};
use base64::{Engine as _, engine::general_purpose};
use std::io::Cursor;
use std::cmp;

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn capture_overlay(app_handle: tauri::AppHandle, window: String) -> Result<String, String> {
    
    // 1. On récupère la fenêtre CIBLE (l'overlay) grâce à son label
    let target_window = app_handle.get_webview_window(&window)
        .ok_or("Fenêtre Overlay introuvable")?;

    // 2. On récupère ses coordonnées physiques
    let window_pos = target_window.outer_position().map_err(|e| e.to_string())?;
    let window_size = target_window.inner_size().map_err(|e| e.to_string())?;

    let win_center_x = window_pos.x + (window_size.width as i32 / 2);
    let win_center_y = window_pos.y + (window_size.height as i32 / 2);

    // 3. Récupération des moniteurs via xcap
    let monitors = Monitor::all().map_err(|e| e.to_string())?;

    // 4. On cherche sur quel écran se trouve l'overlay
    let best_monitor = monitors.into_iter().find(|m| {
        // xcap renvoie des Result, on sécurise avec unwrap_or(0)
        let m_x = m.x().unwrap_or(0);
        let m_y = m.y().unwrap_or(0);
        let m_w = m.width().unwrap_or(0) as i32;
        let m_h = m.height().unwrap_or(0) as i32;

        win_center_x >= m_x && win_center_x < (m_x + m_w) &&
        win_center_y >= m_y && win_center_y < (m_y + m_h)
    });

    if let Some(monitor) = best_monitor {
        // On récupère les coordonnées de l'écran trouvé
        let monitor_x = monitor.x().map_err(|e| e.to_string())?;
        let monitor_y = monitor.y().map_err(|e| e.to_string())?;

        // Capture de tout l'écran
        let screen_image = monitor.capture_image().map_err(|e| e.to_string())?;
        
        // 5. Calcul des coordonnées de découpe (relatives à l'écran)
        let crop_x = (window_pos.x - monitor_x).max(0) as u32;
        let crop_y = (window_pos.y - monitor_y).max(0) as u32;

        // On s'assure de ne pas dépasser les bords de l'image
        let crop_width = cmp::min(window_size.width, screen_image.width().saturating_sub(crop_x));
        let crop_height = cmp::min(window_size.height, screen_image.height().saturating_sub(crop_y));

        if crop_width == 0 || crop_height == 0 {
            return Err("Fenêtre hors de la zone visible".to_string());
        }

        // 6. Crop et encodage
        let dynamic_img = DynamicImage::ImageRgba8(screen_image);
        let cropped = dynamic_img.crop_imm(crop_x, crop_y, crop_width, crop_height);

        let mut buffer = Cursor::new(Vec::new());
        cropped.write_to(&mut buffer, ImageFormat::Jpeg).map_err(|e| e.to_string())?;
        
        let base64_str = general_purpose::STANDARD.encode(buffer.get_ref());
        Ok(format!("data:image/jpeg;base64,{}", base64_str))

    } else {
        Err("Impossible de déterminer le moniteur de l'overlay".to_string())
    }
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
