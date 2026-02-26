// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

use webrtc::api::APIBuilder;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_VP8};
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

use axum::{extract::State as AxumState, http::StatusCode, routing::{get, post}, Router};
use tower_http::cors::CorsLayer;

// ── Shared state types ──────────────────────────────────────────────────────

struct SignalingHandle {
    peer_connection: Arc<RTCPeerConnection>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    _server_port: u16,
}

struct StreamState {
    gst_process: Mutex<Option<std::process::Child>>,
    signaling: TokioMutex<Option<SignalingHandle>>,
}

// ── Axum signaling server (offer/answer) ────────────────────────────────────

#[derive(Clone)]
struct SignalingServerState {
    offer_sdp: Arc<str>,
    peer_connection: Arc<RTCPeerConnection>,
}

async fn get_offer_handler(AxumState(st): AxumState<SignalingServerState>) -> String {
    st.offer_sdp.to_string()
}

async fn post_answer_handler(
    AxumState(st): AxumState<SignalingServerState>,
    body: String,
) -> (StatusCode, String) {
    match RTCSessionDescription::answer(body) {
        Ok(answer) => match st.peer_connection.set_remote_description(answer).await {
            Ok(_) => (StatusCode::OK, "OK".to_string()),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        },
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

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

// ── WebRTC stream commands ─────────────────────────────────────────────────

#[tauri::command]
async fn start_stream(state: tauri::State<'_, StreamState>) -> Result<u16, String> {
    // Prevent double-start
    {
        let sig = state.signaling.lock().await;
        if sig.is_some() {
            return Err("Stream already running".to_string());
        }
    }

    // ── 1. MediaEngine: VP8 with fixed payload type 96
    let mut me = MediaEngine::default();
    me.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: String::new(),
                rtcp_feedback: vec![],
            },
            payload_type: 96,
            ..Default::default()
        },
        RTPCodecType::Video,
    ).map_err(|e| e.to_string())?;

    let api = APIBuilder::new().with_media_engine(me).build();

    // ── 2. PeerConnection
    let peer_connection = Arc::new(
        api.new_peer_connection(RTCConfiguration::default())
            .await
            .map_err(|e| e.to_string())?,
    );

    // ── 3. VP8 track
    let video_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_VP8.to_owned(),
            clock_rate: 90000,
            ..Default::default()
        },
        "video".to_owned(),
        "screen".to_owned(),
    ));

    let rtp_sender = peer_connection
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .map_err(|e| e.to_string())?;

    // Read RTCP to keep the connection alive
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut buf).await {}
    });

    // ── 4. Create offer and wait for ICE gathering
    let offer = peer_connection
        .create_offer(None)
        .await
        .map_err(|e| e.to_string())?;

    let mut gather_complete = peer_connection.gathering_complete_promise().await;
    peer_connection
        .set_local_description(offer)
        .await
        .map_err(|e| e.to_string())?;
    let _ = gather_complete.recv().await;

    let local_desc = peer_connection
        .local_description()
        .await
        .ok_or_else(|| "ICE gathering failed — no local description".to_string())?;
    let offer_sdp: Arc<str> = local_desc.sdp.into();

    // ── 5. UDP socket: bridge GStreamer RTP → WebRTC track
    let video_track_clone = Arc::clone(&video_track);
    let udp_socket = tokio::net::UdpSocket::bind("127.0.0.1:5004")
        .await
        .map_err(|e| format!("UDP bind :5004 failed: {}", e))?;

    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        loop {
            match udp_socket.recv(&mut buf).await {
                Ok(n) => { let _ = video_track_clone.write(&buf[..n]).await; }
                Err(e) => { eprintln!("[webrtc] UDP recv error: {}", e); break; }
            }
        }
    });

    // ── 6. Axum signaling server
    let srv_state = SignalingServerState {
        offer_sdp: offer_sdp.clone(),
        peer_connection: Arc::clone(&peer_connection),
    };

    let app = Router::new()
        .route("/api/offer", get(get_offer_handler))
        .route("/api/answer", post(post_answer_handler))
        .layer(CorsLayer::permissive())
        .with_state(srv_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { let _ = shutdown_rx.await; })
            .await
            .ok();
    });

    // ── 7. Launch GStreamer → VP8 RTP → UDP :5004
    {
        let mut gst_guard = state.gst_process.lock().map_err(|e| e.to_string())?;
        let mut cmd = gst_command()?;
        let child = cmd
            .args([
                "d3d11screencapturesrc", "show-cursor=true",
                "!", "video/x-raw,framerate=30/1",
                "!", "queue",
                "!", "videoconvert",
                "!", "vp8enc", "deadline=1", "target-bitrate=2000000",
                "!", "rtpvp8pay", "mtu=1300", "pt=96",
                "!", "udpsink", "host=127.0.0.1", "port=5004",
            ])
            .spawn()
            .map_err(|e| format!("Failed to start GStreamer: {}", e))?;
        *gst_guard = Some(child);
    } // MutexGuard dropped here — before any await

    // ── 8. Save handle
    let mut sig_guard = state.signaling.lock().await;
    *sig_guard = Some(SignalingHandle { peer_connection, shutdown_tx, _server_port: port });

    Ok(port)
}

#[tauri::command]
async fn stop_stream(state: tauri::State<'_, StreamState>) -> Result<String, String> {
    // Kill GStreamer
    if let Ok(mut guard) = state.gst_process.lock() {
        if let Some(mut child) = guard.take() {
            child.kill().ok();
            child.wait().ok();
        }
    }
    // Close WebRTC + signaling server
    let mut sig_guard = state.signaling.lock().await;
    if let Some(handle) = sig_guard.take() {
        let _ = handle.shutdown_tx.send(());
        handle.peer_connection.close().await.ok();
    }
    Ok("Stream stopped".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(StreamState {
            gst_process: Mutex::new(None),
            signaling: TokioMutex::new(None),
        })
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
