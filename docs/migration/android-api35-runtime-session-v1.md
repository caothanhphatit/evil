# Android API 35 runtime session v1

Date: 2026-07-26

## Result

The Google APIs Android 15 ARM64 emulator is stable enough to reach a new-game
tutorial town. Unlike the earlier API 30 session, the package remained in the
foreground through the title, terms, local chief creation, and tutorial entry.
The tutorial displayed one live Hunter named `Sharon`.

This confirms that an API 35 ARM64 session can support controlled UI actions.
It does not establish any Hunter field/value binding: no live managed Hunter
values were read during this session.

## Environment

- AVD: `evil_hunter_api35`
- image: `system-images;android-35;google_apis;arm64-v8a`
- device model: `sdk_gphone64_arm64`
- Android release/API: `15` / `35`
- package: `com.superplanet.evilhunter`
- package version: `1.411` (`26071501`)
- installed splits: `com.superplanet.evilhunter.apk`, `base_assets.apk`, and
  `config.arm64_v8a.apk`
- Google Play Services version code: `242335038`
- Frida client/server: `17.16.4` / `17.16.4`

The clean run did not have a Frida server running. It remained alive from the
18:05:25 local relaunch through tutorial entry and the later capture setup.

## API 35 schema evidence

The existing reflection-only dumper attached to the tutorial process and wrote:

`reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json`

- bytes: `670307`
- SHA-256: `6cc2faa575ed87567ed2262f2910a372596421d5d3b92264288639faa47da678`
- captured at: `2026-07-26T11:16:43.047065Z`
- missing target types: none

The normalized `record.payload.classes` JSON has SHA-256
`b08193d79e12465b9d378f7d6bd31b5300bb731d552912215656a815ac074816`,
exactly matching the API 30 primary Hunter schema capture. The repeated counts
are `UserData` 527/1019, `HunterDetailPop` 194/71, `SaveData` 4/23,
`HunterData` 109/236, and `HunterLookData` 11/1 for fields/methods.

The capture command was:

```sh
~/.local/share/evil-frida-venv/bin/python \
  tools/runtime/capture-hunter-info-schema.py \
  --adb "$HOME/Library/Android/sdk/platform-tools/adb" \
  --pid 4932 \
  --attach-delay 1 \
  --timeout 25 \
  --action "API 35 tutorial town after creating disposable local chief Codex; Hunter Sharon visible; schema-only attach" \
  --output reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json
```

## Instrumented-session blocker

The schema record completed successfully. About ten seconds later Android
recorded the game process exit as `reason=2 (SIGNALED)`, `status=9`. Logcat first
reported the Frida child as a dead `PhantomProcessRecord`, then the main process
death. There was no Java fatal exception, native fatal signal record, or ANR.

This timing does not prove whether the cause is Android phantom-process policy,
instrumentation detection, or another runtime condition. Because the clean
process was stable and the instrumented process was not, live value capture is
still blocked on a session that remains stable after attachment. Do not infer
values from the visible tutorial UI or from the schema field names.

The emulator was stopped with `adb emu kill` after collecting the evidence.
