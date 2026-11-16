# Développement et livraison

## Branches et commits

Chaque fonctionnalité, correction ou changement documentaire utilise une branche dédiée créée depuis `main` à jour. Utilisez des noms explicites, par exemple `feature/live-debug`, `fix/speech-input` ou `docs/usage`.

Créez des commits ciblés à chaque avancée cohérente. Le message décrit le changement avec un préfixe tel que `feat:`, `fix:`, `docs:` ou `test:`. Poussez la branche aux jalons utiles pour conserver les avancées sur le dépôt distant. Une branche peut contenir du travail en cours ; `main` doit conserver les paliers fonctionnels validés.

## Documentation au fil du travail

Mettez à jour `README.md` et les pages concernées de `doc/` avec les changements de comportement, de configuration, de commandes, d'architecture ou de limites. La documentation fait partie du lot livré ; ne la reportez pas à la fin du projet.

Décrivez séparément les résultats des tests automatisés, les vérifications matérielles et la validation de bout en bout avec ChatGPT. Ne présentez pas une intégration non testée comme validée.

## Vérification avant fusion

Pour un changement de code Rust :

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Ajoutez une vérification ciblée du comportement modifié. Les tests matériels ignorés par défaut doivent être exécutés explicitement sur une machine disposant du microphone ou du moteur requis. Un changement documentaire seul demande une relecture et une vérification des liens et commandes ; il ne nécessite pas de nouveaux tests qui reproduisent le texte.

Avant chaque commit et push, inspectez les fichiers indexés et vérifiez les exclusions de [confidentialité](confidentialite.md). `ForAi.md` est versionné et conserve les règles du projet pour les IA ; son contenu ne doit pas contenir de données sensibles. Le suivi local, les dossiers de travail des IA et les configurations personnelles restent exclus de Git. Utilisez une identité de commit configurée avec une adresse GitHub sans réponse lorsque la confidentialité de l'adresse électronique doit être préservée.

## Fusion et publication

Récupérez les dernières modifications de `main`, intégrez-les à la branche si nécessaire, puis vérifiez le résultat. Fusionnez chaque palier prêt dans `main` avec un commit de fusion (`git merge --no-ff`) pour préserver le regroupement des changements. Respectez les protections du dépôt et utilisez une pull request lorsqu'elles l'exigent.

Poussez `main` après la fusion puis comparez ses identifiants local et distant. Le compte rendu de livraison précise les commits, les contrôles réellement exécutés et les limites restantes. Aucun push forcé, réécriture d'historique publié ou suppression de branche ne fait partie du cycle normal.
