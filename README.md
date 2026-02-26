# Overlay Desktop

Application de bureau permettant de créer des fenêtres overlay transparentes et toujours au premier plan. Construite avec **Tauri 2**, **Vue 3** et **TypeScript**.

## Fonctionnalités

- **Mode Fantôme (Ghost Mode)** : Basculer entre le mode interactif et le mode click-through avec la touche `F8`
- **Fenêtres Transparentes** : Création de fenêtres flottantes transparentes toujours au premier plan
- **Raccourci Global** : Support du raccourci clavier `F8` pour contrôler l'overlay
- **Multi-fenêtres** : Possibilité d'ouvrir plusieurs fenêtres overlay dynamiquement

## Stack Technique

- **Frontend** : Vue 3 (TypeScript) + Vite
- **Backend** : Rust (Tauri 2)
- **Package Manager** : Bun

## Installation et Lancement

Voir [COMMANDES.md](COMMANDES.md) pour les instructions détaillées.

```bash
# Installer Bun : https://bun.com/docs/installation

# Lancement en mode développement
bun tauri dev

# Build de production
bun tauri build
```

## IDE Recommandé

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
