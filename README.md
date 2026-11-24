# WSLCockpit

Application de bureau Windows pour gérer ses distributions WSL2 : état, démarrage/arrêt, terminal, alias, politiques, monitoring, configuration, export/import/clone, suppression, conversion.

Tauri 2 + Svelte, exe portable (pas d'installeur), thème clair/sombre selon Windows, interface en français.

## Utilisation

1. Télécharge `WSLCockpit.exe` (release GitHub, ou artefact `WSLCockpit-portable` de la CI).
2. Lance-le : aucun droit administrateur n'est nécessaire.
3. Fermer la fenêtre la réduit dans la zone de notification ; **Quitter** se trouve dans le menu de l'icône.

SmartScreen peut afficher « Windows a protégé votre ordinateur » car l'exe n'est pas signé : *Informations complémentaires* puis *Exécuter quand même*.

Prérequis : Windows 10/11 avec WSL installé (`wsl --install`) et WebView2 (présent par défaut sur Windows 11).

Réglages (alias, politiques) : `%APPDATA%\WSLCockpit`. L'app vérifie au lancement si une release plus récente existe (sans message d'erreur hors ligne).

## Fonctions

- **Liste** : état, version WSL, distribution par défaut (bouton « Par défaut »), taille du disque virtuel, RAM/CPU de la VM, espace disque des distributions démarrées.
- **Démarrer, arrêter, redémarrer, terminal** (Windows Terminal si présent, sinon console), **Tout arrêter** (`wsl --shutdown`).
- **Alias** : renommage d'affichage local, WSL n'est pas modifié.
- **Politiques** par distribution : manuel, démarrer à l'ouverture, garder en vie.
- **Configuration** : formulaires pour `.wslconfig` (mémoire, processeurs, swap, réseau, `vmIdleTimeout`) et `wsl.conf` de chaque distribution (systemd, utilisateur par défaut, montage automatique). Les valeurs invalides bloquent l'enregistrement, les commentaires et clés inconnues sont conservés, et une sauvegarde `.bak-AAAAMMJJ-HHMMSS` de l'ancien fichier est créée à côté avant chaque écriture. Une distribution arrêtée n'est démarrée qu'après accord. Un redémarrage (`wsl --shutdown`) est nécessaire pour appliquer.

## Opérations sur les distributions

Une seule opération à la fois, avec progression et annulation. Les chemins se saisissent à la main (pas de sélecteur de fichier).

| Opération | Commande WSL | Garde-fous |
|---|---|---|
| Exporter | `wsl --export` | ne remplace jamais un fichier existant, `.tar` obligatoire ; archive partielle supprimée si échec ou annulation |
| Importer | `wsl --import --version 2` | nom unique (lettres, chiffres, `_ - .`), dossier absent ou vide ; import partiel désinscrit si échec ou annulation |
| Cloner | export puis import | archive temporaire à côté du dossier choisi, supprimée ensuite |
| Supprimer | `wsl --unregister` | saisie du nom exact (vérifiée aussi côté Rust), export proposé avant ; **irréversible** |
| Convertir WSL1 / WSL2 | `wsl --set-version` | distribution arrêtée ; peut durer plusieurs minutes |
| Mode sparse | `wsl --manage --set-sparse` | distribution arrêtée ; grisé avec la raison si WSL est trop ancien (seuil 2.0.0) |

Limites : l'état actuel du mode sparse n'est pas affiché ; la progression d'un export/import indique les octets écrits, sans pourcentage ; aucune élévation administrateur (le compactage du disque virtuel n'est pas disponible).

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
