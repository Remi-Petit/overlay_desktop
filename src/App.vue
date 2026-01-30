<script setup lang="ts">
import { ref } from "vue";
import { register } from '@tauri-apps/plugin-global-shortcut';
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

const greetMsg = ref("");
const name = ref("");

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke("greet", { name: name.value });
}


// État actuel : true = on clique à travers (fantôme), false = on clique sur l'app
let isGhostMode = true;

async function setupOverlay() {
    // 1. Au démarrage, on active le mode fantôme
    await invoke('set_ignore_cursor_events', { ignore: true });
    console.log("App démarrée en mode fantôme.");

    // 2. On enregistre le raccourci global (Ici: Alt + O)
    await register('F8', async (event) => {
        if (event.state === "Pressed") {
            // On inverse l'état
            isGhostMode = !isGhostMode;
            
            // On applique le changement à la fenêtre
            await invoke('set_ignore_cursor_events', { ignore: isGhostMode });
            
            // Feedback visuel (optionnel)
            console.log(isGhostMode ? "Mode Fantôme (Click-through)" : "Mode Interactif");
            
            // Astuce UX : Changer l'opacité pour montrer qu'on est en mode interactif
            document.body.style.opacity = isGhostMode ? "0.5" : "1.0";
        }
    });
}

// Lancer la configuration
setupOverlay();


// 2. Fonction pour créer la nouvelle fenêtre
async function openNewWindow() {
  // On crée une nouvelle fenêtre avec un identifiant unique (ex: 'secondary')
  const webview = new WebviewWindow('secondary', {
    url: 'https://tauri.app', // Tu peux mettre une URL externe ou locale (ex: '/#/settings')
    title: 'Ma Seconde Fenêtre',
    width: 600,
    height: 400,
    decorations: true, // On veut probablement des bordures pour cette fenêtre
    alwaysOnTop: false // Pas forcément au-dessus de tout
  });

  // Gestion des erreurs (optionnel)
  webview.once('tauri://error', function (e) {
    console.error('Erreur création fenêtre', e);
  });
}
</script>

<template>
  <main class="container">
    <h1>Welcome to Tauri + Vue</h1>

    <form class="row" @submit.prevent="greet">
      <input id="greet-input" v-model="name" placeholder="Enter a name..." />
      <button type="submit">Greet</button>
    </form>
    
    <div class="row" style="margin-top: 20px;">
        <button @click="openNewWindow">Ouvrir une nouvelle fenêtre</button>
    </div>

    <p>{{ greetMsg }}</p>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
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

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
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

/* Si tu veux un effet de flou (Glassmorphism) */
/* 
#app {
  background-color: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  height: 100vh;
} 
*/

</style>