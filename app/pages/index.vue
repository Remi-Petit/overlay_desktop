<script setup lang="ts">
import { ref } from "vue";
import { register } from '@tauri-apps/plugin-global-shortcut';
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';

let isGhostMode = false;
const lastCapture = ref("");
const gstreamerResult = ref("");
const gstreamerError = ref("");
const gstreamerLoading = ref(false);
const streamRunning = ref(false);
const streamMessage = ref("");
const streamError = ref("");
const streamPort = ref<number | null>(null);
let rtcPeerConnection: RTCPeerConnection | null = null;

async function testGstreamer() {
  gstreamerResult.value = "";
  gstreamerError.value = "";
  gstreamerLoading.value = true;
  try {
    const result = await invoke<string>('test_gstreamer');
    gstreamerResult.value = result;
  } catch (e: any) {
    gstreamerError.value = String(e);
  } finally {
    gstreamerLoading.value = false;
  }
}

async function setupOverlay() {
  await register('F8', async (event) => {
      if (event.state === "Pressed") {
        isGhostMode = !isGhostMode;
        const windows = await getAllWebviewWindows();
        
        for (const win of windows) {
            if (win.label.startsWith('fenetre-')) {
              await invoke('set_ghost_mode', { 
                  label: win.label, 
                  ghost: isGhostMode 
              });
            }
        }
      }
  });
}

setupOverlay();

async function setupWebRtc(port: number) {
  try {
    // Fetch the SDP offer from the Rust signaling server
    const resp = await fetch(`http://127.0.0.1:${port}/api/offer`);
    if (!resp.ok) throw new Error(`Offer fetch failed: ${resp.status}`);
    const offerSdp = await resp.text();

    // Create a peer connection (local-only — no STUN needed)
    rtcPeerConnection = new RTCPeerConnection({ iceServers: [] });

    // When a track arrives, display it in the <video> element
    rtcPeerConnection.ontrack = (event) => {
      const video = document.getElementById('webrtc-video') as HTMLVideoElement;
      if (video && event.streams[0]) {
        video.srcObject = event.streams[0];
        video.play().catch(() => {});
      }
    };

    // Set remote description = the offer from Rust
    await rtcPeerConnection.setRemoteDescription({ type: 'offer', sdp: offerSdp });

    // Create and set local description (our answer)
    const answer = await rtcPeerConnection.createAnswer();
    await rtcPeerConnection.setLocalDescription(answer);

    // Wait for ICE gathering to complete before sending the answer
    if (rtcPeerConnection.iceGatheringState !== 'complete') {
      await new Promise<void>(resolve => {
        const handler = () => {
          if (rtcPeerConnection?.iceGatheringState === 'complete') {
            rtcPeerConnection?.removeEventListener('icegatheringstatechange', handler);
            resolve();
          }
        };
        rtcPeerConnection!.addEventListener('icegatheringstatechange', handler);
        // Fallback timeout
        setTimeout(resolve, 3000);
      });
    }

    // Send the answer SDP to Rust
    await fetch(`http://127.0.0.1:${port}/api/answer`, {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: rtcPeerConnection.localDescription!.sdp,
    });
  } catch (e: any) {
    streamError.value = `WebRTC: ${String(e)}`;
  }
}

async function toggleStream() {
  streamMessage.value = "";
  streamError.value = "";
  try {
    if (streamRunning.value) {
      // Close WebRTC
      if (rtcPeerConnection) {
        rtcPeerConnection.close();
        rtcPeerConnection = null;
      }
      // Clear video
      const video = document.getElementById('webrtc-video') as HTMLVideoElement;
      if (video) video.srcObject = null;

      const result = await invoke<string>('stop_stream');
      streamMessage.value = result;
      streamRunning.value = false;
      streamPort.value = null;
    } else {
      // start_stream returns the signaling server port
      const port = await invoke<number>('start_stream');
      streamPort.value = port;
      streamRunning.value = true;
      streamMessage.value = `Stream démarré (port signaling : ${port})`;

      // Give GStreamer ~1 s to start before connecting WebRTC
      setTimeout(() => setupWebRtc(port), 1000);
    }
  } catch (e: any) {
    streamError.value = String(e);
    streamRunning.value = false;
  }
}

async function openNewWindow() {
  const label = `fenetre-${Date.now()}`; 

  const webview = new WebviewWindow(label, {
    url: '/overlay',
    title: 'Overlay',
    width: 600,
    height: 400,
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: true,
    shadow: true,
  });

  webview.once('tauri://error', function (e) {
    console.error('Erreur:', e);
  });
}
</script>

<template>
  <main class="container">
    <div class="row" style="margin-top: 20px; gap: 10px;">
        <button @click="openNewWindow">Ouvrir une nouvelle fenêtre</button>
        <button @click="testGstreamer" :disabled="gstreamerLoading">
          {{ gstreamerLoading ? 'Test en cours...' : 'Tester GStreamer (sidecar)' }}
        </button>
        <button @click="toggleStream" :style="streamRunning ? 'background: #c0392b; color: white;' : ''">
          {{ streamRunning ? '⏹ Arrêter le stream' : '▶ Test Stream' }}
        </button>
    </div>
    <div v-if="gstreamerResult" style="margin-top: 15px; padding: 10px; background: #1a1a2e; color: #0f0; border-radius: 8px; text-align: left; white-space: pre-wrap; font-family: monospace;">
      ✅ {{ gstreamerResult }}
    </div>
    <div v-if="gstreamerError" style="margin-top: 15px; padding: 10px; background: #2e1a1a; color: #f66; border-radius: 8px; text-align: left; white-space: pre-wrap; font-family: monospace;">
      ❌ {{ gstreamerError }}
    </div>
    <div v-if="streamMessage" style="margin-top: 15px; padding: 10px; background: #1a1a2e; color: #0f0; border-radius: 8px; text-align: left; font-family: monospace;">
      ✅ {{ streamMessage }}
    </div>
    <div v-if="streamError" style="margin-top: 15px; padding: 10px; background: #2e1a1a; color: #f66; border-radius: 8px; text-align: left; white-space: pre-wrap; font-family: monospace;">
      ❌ {{ streamError }}
    </div>
    <div class="preview-box" v-if="lastCapture">
      <h3>Flux Overlay (Analyse YOLO à venir)</h3>
      <img :src="lastCapture" style="width: 100%; border: 2px solid red;" />
    </div>

    <!-- WebRTC stream preview -->
    <div v-if="streamRunning" style="margin-top: 20px; border: 2px solid #396cd8; border-radius: 8px; overflow: hidden; background: #000;">
      <div style="padding: 6px 10px; background: #1a1a3e; color: #aaf; font-size: 0.8em; font-family: monospace;">
        ● LIVE — WebRTC loopback (port {{ streamPort }})
      </div>
      <video
        id="webrtc-video"
        autoplay
        muted
        playsinline
        style="width: 100%; max-height: 400px; display: block;"
      />
    </div>
  </main>
</template>

<style scoped>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

:root, body {
  background-color: transparent !important;
}

</style>