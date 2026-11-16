# VoiceTrigger — contexte local pour les IA

Ce fichier est versionné et doit être mis à jour avec les décisions du propriétaire. `tracking/`, les configurations personnelles et les dossiers de travail des IA restent locaux et exclus de Git.

## Objectif utilisateur

Application Windows en Rust, économe en arrière-plan, qui écoute une phrase personnalisée (par défaut « Dit ChatGPT ») pour lancer une action. L'action principale ouvre une nouvelle conversation ChatGPT, démarre sa dictée puis envoie après trois secondes de silence si les contrôles accessibles de l'application le permettent.

## Consignes

- Conserver la licence MIT et le dépôt `https://github.com/TheGeogeo/VoiceTrigger`.
- Lire les instructions globales de l'utilisateur et les skills obligatoires avant les changements.
- Reprendre depuis `tracking/voice-trigger/TRACKING.md` ; ne pas confondre compilation et validation vocale réelle.
- Pour ce projet, le propriétaire autorise les commits, les push des branches et la fusion des fonctionnalités validées dans `main`. Appliquer le cycle Git ci-dessous sans redemander cette autorisation à chaque avancée.
- Ne jamais conserver d'enregistrement, transcription, secret, jeton ou chemin personnel dans les fichiers destinés au Git.
- Garder les déclencheurs locaux via SAPI ; pas de rendu ou capture audio supplémentaire continus au repos.
- Garder UI Automation limitée à ChatGPT ; ne pas introduire de clics par coordonnées ou de raccourcis globaux.
- Documenter les informations normales et importantes du projet dans `doc/`.
- Vérifier les protections `.gitignore` avant de préparer une publication.
- Après Computer Use, fermer la session de capture pour libérer le contour jaune ; ne pas laisser une capture active après les vérifications.

## Cycle Git obligatoire

1. Actualiser les références distantes et partir d'une branche `main` à jour. Créer une branche dédiée par fonctionnalité ou correction, par exemple `feature/live-debug`, `fix/speech-input` ou `docs/usage`.
2. Faire des commits ciblés à chaque avancée cohérente et vérifiée. Utiliser un message explicite (`feat:`, `fix:`, `docs:`, `test:`). Ne pas attendre la fin du projet pour sauvegarder les avancées dans Git.
3. Compléter `README.md` et les fichiers pertinents dans `doc/` pendant l'implémentation. Les changements de comportement, réglages, commandes, limites et résultats de validation doivent être documentés dans le même lot que la fonctionnalité.
4. Relire le diff et les fichiers réellement indexés. Versionner `ForAi.md` en conservant des consignes génériques sans données sensibles. Garder `tracking/`, les dossiers d'IA, les configurations locales et les enregistrements hors du dépôt. Ne jamais utiliser `git add -f` pour contourner ces exclusions.
5. Exécuter les vérifications adaptées avant de fusionner. Un test de compilation ne prouve pas la détection vocale ni l'intégration ChatGPT ; annoncer les limites réelles. Une régression connue qui empêche la fonctionnalité doit être corrigée avant fusion.
6. Pousser régulièrement la branche de travail quand un jalon utile est prêt. Fusionner dans `main` chaque palier fonctionnel validé, sans attendre que toutes les idées du projet soient terminées. Conserver un commit de fusion (`--no-ff`) pour identifier le lot livré.
7. Récupérer les dernières modifications de `main` avant la fusion. Résoudre les conflits sans écraser le travail d'autrui, puis revérifier le résultat si le code change. Respecter les protections GitHub et passer par une PR si elles l'exigent.
8. Pousser `main` après la fusion et vérifier que le commit local correspond au commit distant. Rapporter la branche, les commits, les vérifications, la fusion et le push réellement effectués.

Ne pas forcer un push, réécrire un historique déjà publié, supprimer des branches ou modifier les protections sans instruction distincte. Conserver la licence MIT. Les règles générales du projet sont décrites dans `doc/developpement.md` ; `ForAi.md` est également publié dans le dépôt.

## Vérification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Les limites et preuves réelles appartiennent au suivi local. Les docs publiques doivent décrire le comportement et les limites sans copier de données de cette machine.
