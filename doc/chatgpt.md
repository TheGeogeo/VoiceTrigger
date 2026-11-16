# ChatGPT integration

## Sequence

The primary action discovers ChatGPT processes and restores or launches its native window. It waits for the window and New chat control, requests a conversation, allows navigation to settle, and invokes **dictation** through Windows UI Automation. Recording is confirmed only when both the recording-stop and recording-cancel controls are available. Wait for the readiness tone before asking the question; the wake phrase is not part of the recorded prompt.

After speech followed by three seconds of silence, VoiceTrigger confirms dictation. ChatGPT performs transcription. VoiceTrigger waits up to 30 seconds for an enabled Send button and invokes it once. The response remains in ChatGPT.

Continuous conversational Voice controls turn-taking itself. VoiceTrigger targets dictation to implement the requested silence delay. OpenAI describes the distinction in its [Voice documentation](https://help.openai.com/en/articles/20001274-gpt-live-faq).

## Requirements and limits

- ChatGPT is installed, signed in, and showing its Chat view.
- The user has already granted ChatGPT microphone access.
- The installed version exposes accessible New chat, Dictate, Confirm dictation, Send, and Cancel controls.
- VoiceTrigger runs at the same privilege level as ChatGPT; administrator access is not requested.

Availability depends on app version and current UI state. No stable public automation contract is assumed. Searches are scoped to visible buttons inside one `ChatGPT.exe` window. The integration does not use coordinate clicks, global shortcuts, or simulated Enter presses.

The installed app has passed native startup/cancellation and simultaneous microphone-monitor tests. Full question transcription and actual message submission have not been exercised. Tests deliberately avoid inspecting existing user drafts or sending a message.

## Recording controls and readiness

In the tested French app, `Arrêter la dictée` stops recording for transcription; `Annuler la dictée` discards it. They must not be mapped to the same action. Specific dictation labels take priority over generic aliases, and duplicate controls with the same preferred label produce an explicit error rather than an arbitrary click.

The action waits up to 10 seconds for a window, 20 seconds for New chat, a 700 ms navigation settling period, 10 seconds for Dictate, and up to 5 seconds for recording controls after invoking it. The readiness tone and silence monitor start only after recording is confirmed. Cancellation or errors abort the recording when its cancel control remains available.

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

Window diagnostics include process and window counts, hidden/restorable main windows, and unavailable process-path queries. Process IDs from a process snapshot allow discovery without depending on privileged executable-path queries. Ambiguous visible windows still require the user to select the intended one.

To run the native recording check explicitly:

```powershell
cargo test --locked native_dictation_starts_and_can_be_cancelled -- --ignored --test-threads=1
```

This opens a conversation, checks recording controls and the concurrent silence monitor, and then cancels. It never invokes Send.

## Failure recovery

Missing or ambiguous controls produce a visible error and stop listening. The integration does not try a different window to send the message. If recording remains open, finish or cancel it manually. If transcription succeeded but Send is unavailable, review the draft and send it yourself.
