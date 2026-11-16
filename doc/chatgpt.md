# ChatGPT integration

## Sequence

The primary action opens or restores the native ChatGPT window, requests a new conversation, and invokes its **dictation** button through Windows UI Automation. Wait for the readiness tone before asking the question; the wake phrase is not part of the recorded prompt.

After speech followed by three seconds of silence, VoiceTrigger confirms dictation. ChatGPT performs transcription. VoiceTrigger waits up to 30 seconds for an enabled Send button and invokes it once. The response remains in ChatGPT.

Continuous conversational Voice controls turn-taking itself. VoiceTrigger targets dictation to implement the requested silence delay. OpenAI describes the distinction in its [Voice documentation](https://help.openai.com/en/articles/20001274-gpt-live-faq).

## Requirements and limits

- ChatGPT is installed, signed in, and showing its Chat view.
- The user has already granted ChatGPT microphone access.
- The installed version exposes accessible New chat, Dictate, Confirm dictation, Send, and Cancel controls.
- VoiceTrigger runs at the same privilege level as ChatGPT; administrator access is not requested.

Availability depends on app version and current UI state. No stable public automation contract is assumed. Searches are scoped to visible buttons inside one `ChatGPT.exe` window. The integration does not use coordinate clicks, global shortcuts, or simulated Enter presses.

End-to-end ChatGPT dictation and submission have not been verified in this development session because no accessible ChatGPT window was available. Recognition and microphone tests do not establish this integration's success.

## Custom button labels

Common English and French accessibility labels are included as language data. For another app version, stop listening, quit VoiceTrigger, then edit these fields in the existing complete `settings.json`:

```json
{
  "microphone_button": "",
  "finish_button": "",
  "send_button": "",
  "cancel_button": ""
}
```

Empty fields use built-in labels. A nonempty field requires an exact case-insensitive accessible name. This fragment is not a replacement configuration. Never map the confirmation field to the control that discards recording.

**Diagnostics** reports known controls without starting recording or retaining conversation content. Confirmation and cancellation may be absent before recording starts, and Send may be disabled while the composer is empty. Diagnostic availability alone does not prove an end-to-end submission.

## Failure recovery

Missing or ambiguous controls produce a visible error and stop listening. The integration does not try a different window to send the message. If recording remains open, finish or cancel it manually. If transcription succeeded but Send is unavailable, review the draft and send it yourself.
