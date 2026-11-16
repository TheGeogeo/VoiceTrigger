# VoiceTrigger

Application Windows en Rust pour lancer une action avec une phrase vocale personnalisée. Le déclencheur par défaut est **« Dit ChatGPT »**.

VoiceTrigger utilise le moteur vocal local de Windows. Une icône dans la zone de notification permet de laisser l'application en arrière-plan, d'arrêter l'écoute et de quitter. Aucun modèle à télécharger ni clé API.

## Démarrage

Prérequis : Windows 10/11, microphone par défaut fonctionnel et moteur de reconnaissance vocale SAPI pour la langue choisie. Rust 1.72 ou supérieur et les outils de compilation MSVC sont nécessaires pour compiler.

```powershell
cargo build --release --locked
.\target\release\voice-trigger.exe
```

1. Modifiez la phrase et choisissez l'action.
2. Cliquez sur **Enregistrer**, puis **Démarrer l'écoute**.
3. Cliquez sur **Arrière-plan**. Fermer ou réduire la fenêtre la masque également.
4. Dites **« Dit ChatGPT »**, attendez le signal de démarrage de la dictée, puis posez votre question.
5. Après votre question, trois secondes de silence valident la dictée et déclenchent l'envoi dans ChatGPT, lorsque ses boutons sont accessibles.

Double-cliquez sur l'icône de notification pour retrouver la fenêtre. Son menu permet d'arrêter l'écoute ou de quitter. Le lancement de l'application n'active pas automatiquement le microphone.

Une seule instance s'exécute à la fois. Relancer l'exécutable fait réapparaître sa fenêtre. Le bouton **Quitter** ferme complètement l'application.

## Actions

- Dicter à ChatGPT dans une nouvelle conversation.
- Ouvrir l'application ChatGPT.
- Ouvrir une adresse HTTP(S) dans le navigateur par défaut.
- Lancer un fichier `.exe`, sans arguments ni interpréteur de commandes.

La dictée ChatGPT dépend des boutons exposés par la version installée. Ouvrez ChatGPT une première fois, connectez-vous, autorisez son microphone et choisissez la vue **Chat**. Le bouton **Diagnostic** vérifie les moteurs vocaux et les contrôles accessibles sans lancer d'enregistrement. Voir [l'intégration ChatGPT](doc/chatgpt.md).

## Documentation

- [Utilisation et dépannage](doc/utilisation.md)
- [Intégration ChatGPT et limites](doc/chatgpt.md)
- [Architecture et consommation](doc/architecture.md)
- [Confidentialité et Git](doc/confidentialite.md)
- [Développement, branches et livraison](doc/developpement.md)

## Développement

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run -- --diagnostic --config .\target\diagnostic-settings.json
```

Le test matériel du microphone est ignoré par défaut. Pour l'exécuter explicitement dans une session Windows avec accès au microphone :

```powershell
cargo test --locked default_microphone_produces_valid_audio_buffers -- --ignored
```

`Cargo.lock` est conservé pour reproduire la compilation de cette application. Le diagnostic en ligne de commande crée un fichier `.diagnostic.txt` à côté de la configuration. L'exécutable release est une application graphique ; utilisez ce fichier pour lire le rapport.

Licence [MIT](LICENSE).
