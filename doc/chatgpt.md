# Intégration ChatGPT

## Fonctionnement

L'action principale ouvre ou restaure la fenêtre native ChatGPT, demande une nouvelle conversation, puis invoque son bouton de **dictée** avec Windows UI Automation. Un signal indique quand commencer à parler. La phrase de déclenchement n'est pas incluse dans la question : attendez ce signal avant de poursuivre.

Après trois secondes de silence, VoiceTrigger invoque le bouton de validation de la dictée. ChatGPT effectue sa transcription. VoiceTrigger attend ensuite jusqu'à 30 secondes un bouton d'envoi actif, puis l'invoque une seule fois. La réponse reste dans ChatGPT.

Le mode vocal conversationnel continu, qui décide lui-même des tours de parole, est différent. VoiceTrigger cible la dictée pour contrôler le délai demandé. OpenAI décrit cette différence dans sa [documentation Voice](https://help.openai.com/en/articles/20001274-gpt-live-faq).

## Conditions

- ChatGPT est installé, connecté et dans la vue Chat.
- Son accès au microphone est déjà autorisé par l'utilisateur.
- La version installée expose les boutons de nouvelle conversation, dictée, validation et envoi à Windows UI Automation.
- VoiceTrigger s'exécute au même niveau de privilège que ChatGPT, sans demander les droits administrateur.

La disponibilité varie selon la version et l'état de l'interface. Aucun contrat public d'automatisation stable n'est supposé. Les recherches sont limitées à une fenêtre du processus `ChatGPT.exe` et aux boutons visibles. Aucun clic par coordonnées, raccourci global ou simulation de touche Entrée n'est utilisé.

## Adapter les libellés

Les libellés français et anglais courants sont intégrés. Pour une autre version, arrêtez l'écoute, fermez VoiceTrigger via son menu, puis modifiez ces champs dans `settings.json` :

```json
{
  "microphone_button": "",
  "finish_button": "",
  "send_button": "",
  "cancel_button": ""
}
```

Un champ vide active les libellés intégrés. Un champ renseigné impose une correspondance exacte avec le nom accessible du bouton, sans distinction de casse. Ces champs s'ajoutent au fichier complet existant ; cet extrait n'est pas une configuration de remplacement. Le bouton de validation ne doit jamais désigner le bouton qui annule l'enregistrement.

Le diagnostic signale la présence des contrôles connus sans lire ni enregistrer le contenu des conversations. La validation et l'annulation peuvent être absentes avant le début d'une dictée ; le bouton d'envoi peut être désactivé quand le champ est vide. Un résultat positif ne prouve pas un envoi vocal de bout en bout.

## En cas d'échec

Un bouton manquant ou ambigu produit une erreur visible et arrête l'écoute. Le programme n'essaie pas d'envoyer dans une autre fenêtre. Si l'enregistrement reste ouvert, terminez-le ou annulez-le manuellement. Si la transcription a abouti mais que l'envoi est introuvable, vérifiez le brouillon et envoyez-le vous-même.
