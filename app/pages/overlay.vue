<template>
  <div class="overlay-container" data-tauri-drag-region></div>
</template>

<style scoped>
/* https://v2.tauri.app/fr/learn/window-customization/#html */
*[data-tauri-drag-region] {
  app-region: drag;
  width: 100vw;
  height: 100vh;
}

:global(html), 
:global(body) {
  margin: 0;
  padding: 0;
  overflow: hidden;
  background-color: red;
}
</style>