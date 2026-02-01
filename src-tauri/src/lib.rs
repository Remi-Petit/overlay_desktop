// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

use xcap::Monitor;
use image::{DynamicImage, ImageFormat}; // ImageFormat au lieu de ImageOutputFormat
use base64::{Engine as _, engine::general_purpose};
use std::io::Cursor;

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn capture_overlay(window: tauri::Window) -> Result<String, String> {
    // 1. Récupérer le facteur d'échelle (ex: 1.5 pour 150%)
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;

    // 2. Coordonnées logiques
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;

    // 3. Conversion en pixels physiques (ceux utilisés par xcap)
    let physical_x = (pos.x as f64 * scale_factor) as u32;
    let physical_y = (pos.y as f64 * scale_factor) as u32;
    let physical_width = (size.width as f64 * scale_factor) as u32;
    let physical_height = (size.height as f64 * scale_factor) as u32;

    // 4. Capture du moniteur
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    // Idéalement, trouver le moniteur où se trouve la fenêtre :
    let monitor = monitors.first().ok_or("Aucun moniteur détecté")?;
    let image = monitor.capture_image().map_err(|e| e.to_string())?;

    let dynamic_img = DynamicImage::ImageRgba8(image);
    
    // 5. Crop avec les coordonnées physiques
    let cropped = dynamic_img.crop_imm(
        physical_x, 
        physical_y, 
        physical_width, 
        physical_height
    );

    // ... reste du code pour l'encodage ...
    let mut buffer = Cursor::new(Vec::new());
    cropped.write_to(&mut buffer, ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    let base64_str = general_purpose::STANDARD.encode(buffer.get_ref());
    
    Ok(format!("data:image/jpeg;base64,{}", base64_str))
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
