# Architecture et consommation

## Composants Rust

- `src/config.rs` : déclencheurs, validation et sauvegarde JSON atomique.
- `src/silence.rs` : état de détection du silence, indépendant des API Windows.
- `src/platform/ui.rs` : fenêtre Win32, contrôles natifs et icône de notification.
- `src/platform/speech.rs` : reconnaissance SAPI avec une grammaire limitée aux déclencheurs.
- `src/platform/audio.rs` : capture WinMM temporaire et calcul RMS.
- `src/platform/chatgpt.rs` : recherche de la fenêtre ChatGPT et invocation des boutons accessibles.
- `src/platform/engine.rs` : écoute et séquence de dictée sur un thread séparé.

## Attente en arrière-plan

L'interface dort dans la boucle de messages Windows. Aucun navigateur embarqué, rendu continu, minuteur graphique ou appel réseau périodique n'est ajouté. Le thread SAPI attend un événement natif par tranches de 100 ms pour pouvoir réagir à l'arrêt. La capture WinMM et les recherches UI Automation sont actives uniquement pendant une action ou un diagnostic.

Le moteur SAPI lui-même consomme des ressources pour reconnaître la phrase. Le projet ne promet ni consommation nulle ni mesure universelle ; la charge dépend du moteur Windows, du microphone et du matériel. Mesurez la version release sur la machine cible.

## Cycle de dictée

1. SAPI détecte une règle et libère son microphone.
2. ChatGPT ouvre une nouvelle conversation et démarre sa dictée.
3. WinMM mesure le microphone par buffers de 50 ms, sans écrire d'audio sur disque.
4. Après de la voix puis trois secondes de silence, la dictée est validée et l'envoi invoqué.
5. Une pause de deux secondes précède la réactivation de SAPI.

Les interfaces COM restent dans leur thread. Les commandes d'arrêt utilisent un indicateur atomique. Les statuts remontent à la fenêtre par canal Rust et message Win32, sans boucle de rafraîchissement. Une erreur d'intégration arrête l'écoute. Les buffers audio sont réinitialisés et libérés à la fin de la dictée.

## Vérification

Les tests couvrent les configurations invalides, les doublons, le remplacement de la sauvegarde et les frontières temporelles du silence. La compilation ne prouve pas la reconnaissance réelle ni la compatibilité d'une version de ChatGPT. Complétez par un test oral et un contrôle de la transcription sur chaque environnement cible.
