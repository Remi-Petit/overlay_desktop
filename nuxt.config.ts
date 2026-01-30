// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },
  ssr: false,

  vite: {
    // Meilleur compatibilité pour la sortie "Tauri CLI"
    clearScreen: false,
    // Activez les variables d'environnement
    // Vous pouvez trouver les variables d'environnements additionnelles sur
    // https://v2.tauri.app/reference/environment-variables/
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      // Tauri requiert un port constant
      strictPort: true,
      // Active le serveur de développement pour être visible par les autres appareils pour le développement mobile
      host: '0.0.0.0',
      hmr: {
        // Utilisez le websocket pour le rechargement à chaud

        protocol: 'ws',
        // Assurez-vous que ce soit disponible sur le réseau
        host: '0.0.0.0',
        // Utilisez un port spécifique pour hmr
        port: 5183,
      },
    },
  },

  modules: [
    '@nuxt/content',
    '@nuxt/hints',
    '@nuxt/image',
    '@nuxt/ui'
  ],
});