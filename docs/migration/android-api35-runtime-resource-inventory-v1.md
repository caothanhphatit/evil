# Android API 35 Runtime Resource Inventory v1

Date: 2026-07-26

## Scope

This pass inspected the locally installed, authorized `1.411` ARM64 package
after a clean API 35 tutorial session. It compared installed APK splits with
the supplied extracted source and listed the app's external files/cache. It
did not inspect production traffic, credentials, cookies, account secrets, or
live managed object values.

The machine-readable record is
`reverse-engineering/evidence/android-api35-runtime-resource-inventory-v1.json`.

## What API 35 added to the evidence set

- The installed split set is three files: `base.apk`, `split_base_assets.apk`,
  and `split_config.arm64_v8a.apk`; their hashes and byte sizes are recorded in
  the evidence envelope.
- The runtime writes
  `files/il2cpp/Metadata/global-metadata.dat` to external app storage. Its
  SHA-256 is `ebbadaf6...b63e`, byte-identical to the supplied source metadata
  and the APK copy. This is a duplicate extraction, not new metadata.
- The runtime writes Unity's version GUID (`unity.ver`) and an opaque
  `UnityShaderCache`. No gameplay AssetBundle or new StreamingAssets payload
  appeared in external storage during this session.
- Addressables is local (`2.8.1`): `assets/aa/catalog.bin`, `catalog.hash`,
  `settings.json`, one MonoScripts bundle, and six localization bundles are
  packaged in `split_base_assets.apk`. No remote content URL was exposed by
  `settings.json`.

## Interpretation boundary

The API 35 run did not reveal an additional compressed Hunter/gameplay payload
that is absent from the supplied APK extraction. The local Addressables catalog
is useful for deterministic asset inventory, but `catalog.bin` strings alone do
not establish semantic UI, skill, trait, or runtime-state bindings. Those remain
subject to the serialized scene and controlled value-capture evidence already
documented in the Hunter reports.

## Reproduction commands

```sh
adb shell pm path com.superplanet.evilhunter
adb pull <installed-split-path> /tmp/evil-api35-runtime-inventory/apk/
unzip -l /tmp/evil-api35-runtime-inventory/apk/split_base_assets.apk
adb shell find /sdcard/Android/data/com.superplanet.evilhunter -type f
adb pull /sdcard/Android/data/com.superplanet.evilhunter/files/il2cpp/Metadata/global-metadata.dat /tmp/
```

The emulator session was API 35 ARM64, package `com.superplanet.evilhunter`,
version `1.411` (`26071501`), AVD `evil_hunter_api35`.
