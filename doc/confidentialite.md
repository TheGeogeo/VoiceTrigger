# Confidentialité et Git

La détection des déclencheurs utilise le moteur SAPI installé dans Windows. VoiceTrigger ne demande aucune clé API, n'intègre aucun service cloud de reconnaissance et n'envoie aucun enregistrement à son propre serveur. Les buffers utilisés pour mesurer le silence restent en mémoire et ne sont pas sauvegardés.

Lorsque la dictée ChatGPT démarre, **ChatGPT** traite l'audio et la question selon ses propres services, réglages et conditions. La détection locale du déclencheur ne rend pas la transcription ChatGPT locale.

Les réglages contiennent vos phrases, vos URL et éventuellement des chemins locaux. Le fichier est placé par défaut dans `%APPDATA%\VoiceTrigger`, hors du dépôt. Les diagnostics ne contiennent ni transcription ni texte de conversation ; les rapports peuvent contenir le chemin local du fichier de configuration. Ne les publiez pas sans vérification.

Le `.gitignore` exclut notamment :

- les sorties de compilation et de distribution ;
- les configurations personnelles, diagnostics, journaux et fichiers temporaires ;
- les fichiers `.env`, clés privées et certificats ;
- les dossiers de travail Claude, Codex et agents ;
- `ForAi.md`, `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `AI.md` et `*.ai.md` ;
- le suivi local `tracking/` et les réglages d'éditeur.

`Cargo.lock`, les sources, `README.md`, `doc/` et la licence MIT sont destinés au dépôt. Un `.gitignore` ne retire pas un secret déjà suivi par Git : vérifiez toujours `git diff --cached` avant publication. Les exemples de documentation doivent rester génériques, sans jeton, chemin personnel ou donnée de conversation.
