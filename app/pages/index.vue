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

async function toggleStream() {
  streamMessage.value = "";
  streamError.value = "";
  try {
    if (streamRunning.value) {
      const result = await invoke<string>('stop_stream');
      streamMessage.value = result;
      streamRunning.value = false;
    } else {
      const result = await invoke<string>('start_stream');
      streamMessage.value = result;
      streamRunning.value = true;
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