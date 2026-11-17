# Live debugging

Click **Start listening**, then say the configured phrase. **Live debug** is enabled by default on **Diagnostics > Activity** while that view is visible. The activity page shows input level, incoming audio updates, and a journal timestamped from application startup.

## Reading the journal

| Entry | Meaning |
| --- | --- |
| `Input: ...` | The actual microphone attached to SAPI. Check that it is the device you are speaking into. |
| `Armed trigger: ...` | An exact configured phrase and its action. |
| `Microphone stream started` | The audio stream has started. |
| `Input level: ...%` | Audio is arriving. The bar and elapsed time update even when the level is low. |
| `Sound detected` / `Sound ended` | SAPI detected a sound boundary. |
| `Speech detected` | The recognizer is matching speech against configured phrases. |
| `Partial match` | A provisional hypothesis; no action has started yet. |
| `Recognized` | The recognized phrase and its confidence. |
| `Trigger detected` | A configured phrase was accepted as a trigger. |
| `Rejected` | A hypothesis was rejected, including when confidence is too low. |
| `Speech result contained no phrase` / `Speech result contained no text` | SAPI returned an empty result. Listening continues and no action runs. |
| `Speech detected, but no configured phrase matched` | Speech was detected without a valid trigger match. |
| `Running action` | The accepted trigger is starting its configured action. |
| `Error` | A stage failed; its details remain in the journal and listening stops. |

The grammar remains restricted to configured triggers. This is not a general transcript of everything said. A ChatGPT action can fail after successful phrase recognition; the journal keeps those stages separate.

SAPI confidence is categorical: `1` means high, `0` means normal, and `-1` means low. It is not an error code or a probability. High and normal recognition can trigger an action; low-confidence matches are rejected.

## When “Ok GPT” does not react

1. Check the listening status after **Start listening**.
2. Find **Armed trigger: 'Ok GPT'** in the journal.
3. Check **Input:** and whether the input bar changes when you speak.
4. If audio is absent, check the default microphone and desktop-app microphone access in Windows.
5. If sound appears but recognition is rejected, check the language and try speaking clearly or using a longer phrase.
6. If **Trigger detected** appears but ChatGPT does not open, read the subsequent action error and [integration guidance](chatgpt.md).

The action log now shows each stage: restoring the app, waiting for its window, creating a conversation, waiting for navigation, starting dictation, confirming recording controls, waiting for silence, transcribing, and sending. An Invoke call alone is not treated as proof that recording started.

## Unexpected application termination

An intermittent crash was reproduced in the SAPI text conversion: a null text pointer reached `wcslen`, producing Windows exception `0xc0000005` in `ucrtbase.dll`. The conversion now checks for null before reading. Phrase metadata is also checked because Microsoft's [GetPhrase contract](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms718433(v=vs.85)) permits a null phrase when nothing was recognized. Empty or whitespace-only results cannot run an action, even with a valid rule and high confidence.

This fault depends on a recognition result, rather than a fixed listening timeout. It can occur after dictation when trigger listening resumes. There is no 90-second application exit timer. The bounded two-minute dictation timeout cancels recording; it does not terminate VoiceTrigger.

If a newly built version still closes unexpectedly, record whether it was waiting for a trigger, recording, transcribing, or resuming listening. In **Event Viewer > Windows Logs > Application**, find the matching **Application Error** entry for `voice-trigger.exe`. The exception code, faulting module and offset help distinguish a recurring native crash from a different cause. Review any exported report for private paths before sharing it. The memory-only debug journal does not survive process termination.

## Resource use and privacy

The input level uses events from the existing SAPI recognizer. No second permanent capture is added. Level updates are limited to about five per second. Hiding the window, leaving the Activity view, or disabling **Live debug** removes frequent level, sound, and hypothesis events while keeping trigger recognition active.

During ChatGPT dictation, the level uses the existing WinMM silence-detection buffers. SAPI and WinMM percentages use different calibration and do not represent recognition confidence.

The journal keeps the latest 100 entries in memory, up to 512 characters per message. **Clear log** empties it; quitting discards it. Nothing is saved or transmitted. Important state changes and errors remain available when frequent debug details are disabled.

## Recognition fixes

Two independent startup problems prevented recognition in the initial implementation:

- `SetInput(NULL)` did not attach a microphone to the in-process recognizer. A real default input token is now attached and checked before listening is announced. See Microsoft's [SAPI input requirement](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms718917(v=vs.85)).
- `SetRuleState(NULL)` only affects programmatically created top-level rules marked active at creation. The initial rules lacked that attribute. Each configured rule is now explicitly activated by ID, and startup verifies that a grammar is active. See Microsoft's [rule activation reference](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms718922(v=vs.85)).

Regression tests cover input attachment, real audio events, and dispatch of an emulated configured phrase. Emulation verifies grammar and event routing without claiming real spoken accuracy or running an external action.

```powershell
cargo test --locked -- --include-ignored --test-threads=1
```

These integration tests require the installed French SAPI engine and an accessible microphone. Actual spoken recognition and successful ChatGPT submission remain separate target-environment checks.

The ignored `listener_survives_extended_audio_and_restarts` test listens for 110 seconds with live debug, drops the recognizer, then starts another 110-second session without debug. It uses the real microphone, discards recognized events without running actions, and saves no audio or text. It adds about 220 seconds to the full suite. To run only the sustained-listening check against optimized code:

```powershell
cargo test --release --locked listener_survives_extended_audio_and_restarts -- --ignored --test-threads=1 --nocapture
```

Null text, null phrase metadata, empty-result dispatch, and ownership of valid COM allocations are covered by ordinary tests. The null-text regression reproduced the access violation before the repair.
