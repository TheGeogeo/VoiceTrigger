# Utilisation et dépannage

## Configuration

Les réglages se trouvent dans `%APPDATA%\VoiceTrigger\settings.json`. L'interface permet de modifier les déclencheurs, leur action, la langue SAPI, le seuil du silence et le chemin facultatif de ChatGPT. Pour utiliser une configuration indépendante :

```powershell
.\target\release\voice-trigger.exe --config .\settings.json
```

Sélectionnez un déclencheur pour le modifier. **Appliquer à la sélection** met à jour la liste ; **Ajouter** crée une nouvelle ligne ; **Enregistrer** conserve la liste et les réglages sur disque. L'enregistrement applique aussi les champs de la ligne sélectionnée. Arrêtez l'écoute avant toute modification.

Les phrases doivent être distinctes, sans tenir compte de la casse et des espaces répétés. La liste accepte de 1 à 32 phrases. Pour ChatGPT, les prononciations « chat gé pé té » et « chat g p t » sont également ajoutées au vocabulaire de reconnaissance lorsque la phrase contient « ChatGPT ».

## Microphone et silence

VoiceTrigger et ChatGPT doivent utiliser le même microphone par défaut. L'écoute permanente recherche uniquement les phrases configurées avec SAPI. Pendant une dictée ChatGPT, SAPI est libéré et une capture WinMM mesure l'énergie du microphone, sans conserver l'enregistrement.

Après au moins 150 ms de son au-dessus du seuil, trois secondes sous le seuil déclenchent la validation. Le seuil par défaut est `0.015`, sur une amplitude normalisée de 0 à 1. Augmentez-le si le bruit empêche l'envoi ; diminuez-le si votre voix n'est pas détectée. Les bruits peuvent être pris pour de la voix : utilisez de préférence un casque dans une pièce calme.

Une dictée sans voix est annulée après 15 secondes. Une dictée qui dépasse 120 secondes est annulée. **Arrêter** annule l'enregistrement en cours ; après transcription, il laisse le texte en brouillon sans l'envoyer. Si les boutons ont changé, terminez ou annulez manuellement dans ChatGPT.

## Erreurs courantes

- **Aucun moteur SAPI pour cette langue** : installez la reconnaissance vocale correspondant à la langue dans Windows. Le pack d'affichage seul et les voix de synthèse ne garantissent pas la présence du moteur SAPI.
- **Microphone indisponible** : vérifiez le périphérique d'entrée par défaut, son accès pour les applications de bureau et qu'aucune autre application n'en demande l'accès exclusif.
- **Phrase non détectée** : vérifiez la langue, prononcez distinctement la phrase et essayez un déclencheur de plusieurs mots. Le moteur peut manquer une phrase ou produire un faux déclenchement.
- **ChatGPT introuvable** : ouvrez ChatGPT une première fois ou indiquez le chemin de son exécutable. VoiceTrigger peut réutiliser le chemin du processus ChatGPT déjà démarré.
- **Plusieurs fenêtres ChatGPT** : placez la fenêtre voulue au premier plan avant de prononcer le déclencheur.
- **Contrôle ChatGPT introuvable** : consultez [chatgpt.md](chatgpt.md), utilisez Diagnostic et vérifiez la vue Chat.

Après une erreur d'intégration, l'écoute s'arrête pour éviter un nouveau déclenchement pendant un enregistrement resté ouvert. Corrigez le problème puis cliquez sur **Démarrer l'écoute**.
