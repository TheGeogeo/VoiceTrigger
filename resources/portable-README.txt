VoiceTrigger @VERSION@
Windows x64 portable release

Getting started
1. Extract the complete ZIP into a folder you can write to.
2. Run voice-trigger.exe.
3. Review your triggers and Settings, then choose Save configuration.
4. Choose Start listening. The factory trigger is "Ok GPT".
5. Wait for the readiness tone before dictating your question.
6. Three seconds of silence after speech submit the ChatGPT transcription.

Requirements
- Windows 10 or Windows 11, x64.
- Microsoft Visual C++ v14 runtime, x64. Official downloads:
  https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist
- A working default microphone and a Windows SAPI recognizer for the selected
  language. The initial recognition language is French (France).
- The signed-in ChatGPT desktop app, with microphone permission, for dictation.

Use Diagnostics > Activity for recognition events and microphone levels.
Use Diagnostics > System checks > Run checks to inspect the integration.
Hide to tray keeps listening active. File > Quit exits the application.

Settings are stored in %APPDATA%\VoiceTrigger, outside this download.
Existing custom trigger phrases are preserved. No API key is required.
ChatGPT processes dictation through its own services and settings.

The doc folder contains the usage, privacy, interface and troubleshooting guides.
LICENSE contains the project's MIT license. Third-party notices and original
license files are included in THIRD-PARTY-NOTICES.md and licenses/.

Source and issues: https://github.com/TheGeogeo/VoiceTrigger
