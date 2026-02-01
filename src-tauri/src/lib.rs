// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use crabgrab::prelude::*;
use image::{DynamicImage, ImageBuffer, Rgba};
use base64::{engine::general_purpose, Engine as _};
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
async fn capture_specific_window(label: String) -> Result<String, String> {
    // 1️⃣ Autorisation de capture
    let token = match CaptureStream::test_access(false) {
        Some(token) => token,
        None => {
            CaptureStream::request_access(false)
                .await
                .ok_or("Permission de capture refusée")?
        }
    };

    // 2️⃣ Récupération des fenêtres normales
    let content = CapturableContent::new(CapturableContentFilter::NORMAL_WINDOWS)
        .await
        .map_err(|e| e.to_string())?;

    // 3️⃣ Recherche de la fenêtre par titre
    let window = content
        .windows()
        .find(|w| w.title() == label)
        .ok_or(format!("Fenêtre '{}' introuvable", label))?;

    // 4️⃣ Configuration (on laisse CrabGrab choisir le meilleur format)
    let config = CaptureConfig::with_window(window, CaptureStream::supported_pixel_formats()[0])
        .map_err(|e| e.to_string())?;

    // 5️⃣ Capture d'un frame (utilisation d'un channel borné pour la sécurité)
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);

    let mut stream = CaptureStream::new(token, config, move |event| {
        if let Ok(StreamEvent::Video(frame)) = event {
            let _ = sender.try_send(frame); // On envoie si le canal est vide
        }
    }).map_err(|e| e.to_string())?;

    // 6️⃣ Attente du frame
    let frame = receiver
        .recv_timeout(std::time::Duration::from_millis(1000))
        .map_err(|_| "Timeout capture image")?;

    stream.stop().ok();

    // 7️⃣ Extraction du Bitmap avec les types génériques explicites
    let bitmap = frame.get_bitmap().map_err(|e| format!("{:?}", e))?;
    
    let (width, height, flat_data) = match bitmap {
        FrameBitmap::BgraUnorm8x4(data) => {
            let w = data.width as u32;
            let h = data.height as u32;
            let mut pixels = Vec::with_capacity((w * h * 4) as usize);
            
            // CrabGrab retourne souvent du BGRA (Windows/macOS), image-rs veut du RGBA
            for p in data.data.iter() {
                pixels.push(p[2]); // R
                pixels.push(p[1]); // G
                pixels.push(p[0]); // B
                pixels.push(p[3]); // A
            }
            (w, h, pixels)
        },
        _ => return Err("Format de pixel non supporté par cet exemple (attendu: Bgra)".into()),
    };

    // 8️⃣ Création de l'image
    let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, flat_data)
        .ok_or("Buffer invalide")?;

    // 9️⃣ Encodage PNG → base64 (On renomme la variable pour éviter le conflit avec la crate)
    let mut png_bytes: Vec<u8> = Vec::new();
    DynamicImage::ImageRgba8(image_buffer)
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    // Utilisation explicite du chemin de la crate pour éviter toute ambiguïté
    let b64_output = general_purpose::STANDARD.encode(png_bytes);

    Ok(format!("data:image/png;base64,{}", b64_output))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_ghost_mode, capture_specific_window])
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
