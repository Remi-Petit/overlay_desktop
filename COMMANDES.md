# Mise en place

## Installer bun
https://bun.com/docs/installation

## Installation des packages
bun tauri dev

## Lancement du projet
bun tauri dev

## Build du projet
bun tauri dev


# Télécharger la version desktop
Après le build, on peut aller dans "src-tauri/target/release/bundle".
Si vous êtes sur Windows, il faut installer la version "msi" ou "nsis".


# Pour donner le contexte du projet à l'IA
cargo install code2prompt
code2prompt . > contexte_projet.md








# Initialisation du projet
bun x nuxi@latest init . --force
bun add -D @tauri-apps/cli