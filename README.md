# WSLCockpit

Application de bureau Windows pour gérer ses distributions WSL2 : état, démarrage/arrêt, terminal, alias, politiques, monitoring.

Tauri 2 + Svelte, exe portable (pas d'installeur), thème clair/sombre selon Windows, interface en français.

## Utilisation

1. Télécharge `WSLCockpit.exe` (release GitHub, ou artefact `WSLCockpit-portable` de la CI).
2. Lance-le : aucun droit administrateur n'est nécessaire.
3. Fermer la fenêtre la réduit dans la zone de notification ; **Quitter** se trouve dans le menu de l'icône.

SmartScreen peut afficher « Windows a protégé votre ordinateur » car l'exe n'est pas signé : *Informations complémentaires* puis *Exécuter quand même*.

Prérequis : Windows 10/11 avec WSL installé (`wsl --install`) et WebView2 (présent par défaut sur Windows 11).

Réglages (alias, politiques) : `%APPDATA%\WSLCockpit`. L'app vérifie au lancement si une release plus récente existe (sans message d'erreur hors ligne).

## Développement

```
npm install
npm test                                                  # tests des parseurs (cargo test --workspace)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
npm run check                                             # svelte-check
```

Les parseurs de `wsl.exe` vivent dans `crates/wsl-core` et se testent sous Linux ; l'app Tauri est dans `src-tauri/`.

## Licence

MIT, voir [LICENSE](./LICENSE).
