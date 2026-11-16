# Usage and troubleshooting

## Settings

The default configuration is `%APPDATA%\VoiceTrigger\settings.json`. The interface edits triggers, actions, SAPI language, silence threshold, and an optional ChatGPT executable path. To use a separate configuration:

```powershell
.\target\release\voice-trigger.exe --config .\settings.json
```

Select a trigger to edit it. **Apply to selected trigger** updates its row; **Add** creates a new row; **Save** persists the list and settings. Saving also applies the selected row's fields. Stop listening before changing settings.

Keep between 1 and 32 distinct phrases. Matching ignores letter case and repeated spaces. Phrases containing “ChatGPT” also receive the configured French pronunciation variants “chat gé pé té” and “chat g p t”.

## Live feedback

Enable **Live debug** to see the selected microphone, input level, audio activity, recognized or rejected phrases, and action errors. At startup, **Armed trigger** lists the phrases being watched. A successful match produces **Recognized**, **Trigger detected**, and **Running action** entries. See [live debugging](debug.md).

## Microphone and silence

VoiceTrigger and ChatGPT must use the same default microphone. SAPI listens only for configured trigger phrases. During ChatGPT dictation, SAPI releases its input and a temporary WinMM capture measures microphone energy without retaining audio.

After at least 150 ms above the threshold, three seconds below it confirm dictation. The default threshold is `0.015` on normalized amplitude from 0 to 1. Raise it if background noise prevents completion; lower it if your voice is missed. This is energy detection, so noise can count as speech. A headset and a quiet room help.

An empty recording is cancelled after 15 seconds. A recording longer than 120 seconds is cancelled. **Stop** cancels active recording; after transcription, it leaves the draft unsent. If ChatGPT's controls change, finish or cancel manually inside ChatGPT.

## Common problems

- **No SAPI recognizer is installed for this language:** install its Windows speech recognition components. A display-language pack or text-to-speech voice alone does not guarantee a SAPI recognizer.
- **Microphone unavailable:** check the default input, desktop-app microphone access, and whether another application requests exclusive access.
- **No input level:** confirm the device named by **Input:** is the microphone you are using. Verify Windows input settings and restart listening after device changes.
- **Phrase rejected:** check the recognition language, pronounce the configured phrase clearly, and try a longer phrase. Recognition can produce misses and false matches.
- **No ChatGPT window:** open ChatGPT once or supply its executable path. VoiceTrigger can reuse the path of an existing ChatGPT process.
- **Multiple ChatGPT windows:** bring the intended window to the foreground before saying the trigger.
- **ChatGPT control unavailable:** use **Diagnostics**, select the Chat view, and read [ChatGPT integration](chatgpt.md).

Integration errors stop listening to avoid retriggering while recording may still be open. Correct the problem, then click **Start listening** again.
