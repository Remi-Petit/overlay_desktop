// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, Emitter};
use crabgrab::prelude::*;

#[tauri::command]
async fn capture_overlay(app: AppHandle, _window: WebviewWindow) -> Result<(), String> {
    let token = match CaptureStream::test_access(false) {
        Some(token) => token,
        None => CaptureStream::request_access(false).await.ok_or("Accès refusé")?
    };
    let filter = CapturableContentFilter::DISPLAYS;
    let content = CapturableContent::new(filter).await.unwrap();
    let display = content.displays().next().ok_or("Aucun écran détecté")?;
    let config = CaptureConfig::with_display(display, CapturePixelFormat::Bgra8888);

    let handle = app.clone();
    
    let mut stream = CaptureStream::new(token, config, |stream_event| {
        println!("result: {:?}", stream_event);
    }).unwrap();

    // CORRECTION : .detach() n'existe pas, on "oublie" simplement le stream 
    // ou on le stocke dans le State de Tauri pour pouvoir l'arrêter plus tard.
    Box::leak(Box::new(stream)); 

    Ok(())
}

#[tauri::command]
fn set_ghost_mode(app_handle: tauri::AppHandle, label: String, ghost: bool) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_shadow(!ghost).map_err(|e| e.to_string())?;
        window.set_ignore_cursor_events(ghost).map_err(|e| e.to_string())?;
    }
    Ok(())
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
