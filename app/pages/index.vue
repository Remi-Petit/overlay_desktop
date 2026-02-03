<script setup lang="ts">
import { ref, onMounted, onUnmounted, useTemplateRef } from 'vue';
import { register } from '@tauri-apps/plugin-global-shortcut';
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';

let isGhostMode = false;
let isCapturing = false; // Drapeau pour contrôler la boucle de capture
const canvasRef = useTemplateRef('canvasRef');

// On stocke le label de la fenêtre overlay active pour savoir qui capturer
const activeOverlayLabel = ref<string | null>(null);

const startCaptureLoop = async () => {
  if (isCapturing) return;
  isCapturing = true;

  const loop = async () => {
    // 1. Arrêt si le composant est démonté
    if (!isCapturing) return;

    // 2. Si pas de fenêtre cible, on attend un peu
    if (!activeOverlayLabel.value) {
      setTimeout(() => requestAnimationFrame(loop), 500);
      return;
    }

    try {
      // 3. Appel à Rust (Mode Binaire)
      // On attend un ArrayBuffer, pas un JSON !
      const response = await invoke<ArrayBuffer>('capture_overlay', { 
        targetLabel: activeOverlayLabel.value 
      });
      
      // 4. Décodage manuel des octets (Ultra rapide)
      const dataView = new DataView(response);
      
      // Les 4 premiers octets sont la Largeur (Little Endian)
      const width = dataView.getUint32(0, true);
      // Les 4 suivants sont la Hauteur
      const height = dataView.getUint32(4, true);
      
      // Le reste, ce sont les pixels (à partir de l'octet 8)
      // Uint8ClampedArray crée une "vue" sur la mémoire sans la copier
      const pixelData = new Uint8ClampedArray(response, 8);

      // 5. Dessin sur le Canvas
      const canvas = canvasRef.value;
      if (canvas) {
        // Redimensionnement seulement si nécessaire
        if (canvas.width !== width || canvas.height !== height) {
          canvas.width = width;
          canvas.height = height;
        }

        const ctx = canvas.getContext('2d');
        if (ctx) {
            // Création de l'image depuis les données binaires
            const imageData = new ImageData(pixelData, width, height);
            ctx.putImageData(imageData, 0, 0);
        }
      }

    } catch (error) {
      // On ignore silencieusement les erreurs de frame pour ne pas spammer la console
      // console.warn("Frame sautée:", error);
    }

    // 6. Boucle infinie fluide (aussi vite que possible)
    requestAnimationFrame(loop);
  };

  loop();
};

// --- LIFECYCLE VUE ---
onMounted(() => {
  startCaptureLoop();
});

onUnmounted(() => {
  isCapturing = false; // Arrête proprement la boucle
});

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

async function openNewWindow() {
  const label = `fenetre-${Date.now()}`;
  activeOverlayLabel.value = label;

  const webview = new WebviewWindow(label, {
    url: '/overlay',
    title: label,
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
    <div class="row" style="margin-top: 20px;">
        <button @click="openNewWindow">Ouvrir une nouvelle fenêtre</button>
    </div>
    <div class="preview-box">
      <h3>Flux Overlay (Analyse YOLO à venir)</h3>
      <canvas ref="canvasRef" class="overlay-canvas"></canvas>
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