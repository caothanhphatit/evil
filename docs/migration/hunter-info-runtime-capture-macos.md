# Hunter Info Runtime Capture From macOS

## Scope

`tools/runtime/hunter-info-runtime-dump.js` captures IL2CPP schema metadata for
`HunterData`, `HunterLookData`, `UserData`, `SaveData`, and `HunterDetailPop`.
It uses exported IL2CPP reflection APIs and emits JSON through Frida `send()`.
It does not read arbitrary managed object addresses, save values, credentials,
network traffic, or the original production backend.

Use only a test account and a device/build you are authorized to inspect.

## Install an XAPK split set

Install Android platform tools and Frida CLI on the macOS host. The Frida CLI
version must match `frida-server` on the Android ARM64 device.

```sh
brew install android-platform-tools
python3 -m pip install --user frida-tools
adb devices
```

Extract the XAPK to a dedicated temporary directory, inspect its APK list, and
install all base/config splits together:

```sh
capture_dir="$(mktemp -d /tmp/evil-xapk.XXXXXX)"
unzip /path/to/game.xapk -d "$capture_dir"
find "$capture_dir" -maxdepth 1 -name '*.apk' -print | sort
adb install-multiple -r "$capture_dir"/*.apk
```

If the XAPK includes an OBB directory, copy only the package-specific OBB files
after confirming the package name from `manifest.json` or the base APK. Do not
connect the rebuild to the original service or copy production account data.

## Start frida-server on a physical Android device

Use a rooted, authorized test device. Download the matching Android ARM64
`frida-server`, then push and start it:

```sh
adb shell getprop ro.product.cpu.abi
frida --version
adb push /path/to/frida-server /data/local/tmp/frida-server
adb shell su -c 'chmod 755 /data/local/tmp/frida-server'
adb shell su -c '/data/local/tmp/frida-server >/dev/null 2>&1 &'
frida-ps -U
```

Resolve the package identifier before capture:

```sh
adb shell pm list packages | rg -i 'evil|hunter'
```

## Capture

Prefer the host wrapper because it writes a clean JSON evidence envelope with
package, device, Frida, timestamp, action, script, and target metadata:

```sh
python3 tools/runtime/capture-hunter-info-schema.py \
  --adb "$HOME/Library/Android/sdk/platform-tools/adb" \
  --launch \
  --action "Cold launch; no Hunter UI interaction" \
  --output reverse-engineering/evidence/hunter-info-runtime-schema.json
```

The wrapper requires the Python environment containing the matching `frida`
package. Use repeated `--target-type TYPE_NAME` arguments for a reviewed nested
schema pass. Supplying target types replaces the five default targets.

Use `--target-assembly ACTk.Runtime.dll` with reviewed ACTk type names when
auditing the on-device obscured PlayerPrefs boundary. This still captures schema
only; it does not decrypt or export private values.

The script retries for up to 20 seconds while the IL2CPP domain and
`Assembly-CSharp.dll` initialize. On protected builds, attaching a few seconds
after a normal launch can be more reliable than spawn-time injection.

For an Android Emulator Google APIs image that supports `adb root`, run the
server in a dedicated terminal instead of using `su`:

```sh
adb root
adb wait-for-device
adb push /path/to/frida-server /data/local/tmp/frida-server
adb shell chmod 755 /data/local/tmp/frida-server
adb shell /data/local/tmp/frida-server
```

The physical-device workflow remains preferred for stable interactive Hunter
value capture.

Spawn the app so the script can wait for `libil2cpp.so`, then save Frida's
message stream:

```sh
frida -U -f PACKAGE.ID \
  -l tools/runtime/hunter-info-runtime-dump.js \
  > hunter-info-schema.log
```

For an already-running authorized process:

```sh
frida -U -n PROCESS_NAME \
  -l tools/runtime/hunter-info-runtime-dump.js \
  > hunter-info-schema.log
```

The raw CLI stream is a log, not guaranteed JSONL. The useful record has
`kind: "hunter-info-schema"`. Prefer the host wrapper for machine-readable
evidence. Preserve `missing` entries as unresolved evidence; do not fill them
from filename order or assumptions.

## Controlled before/after save capture

This dumper records schema, not player values, so schema output should remain
identical across save changes. Use the following controlled procedure to prove
that boundary and to prepare a later, separately reviewed value-capture tool:

1. Use a disposable test account and record app version, package ID, device ABI,
   Frida version, and UTC timestamp.
2. Capture `before.jsonl` immediately before one known action, such as renaming
   one test Hunter or changing one appearance slot.
3. Perform exactly that action in-game and allow the game to complete its normal
   local save flow.
4. Restart the app and capture `after.jsonl` with the same script and versions.
5. Compare normalized `hunter-info-schema` payloads. Any field/value diff means
   the capture procedure or runtime build changed; this script itself does not
   establish value bindings.

Do not infer a save field binding merely because a schema field name resembles
the UI action. A value binding needs a separate authorized capture with typed,
class-specific access and repeatable before/after evidence.

## macOS ARM64 IL2CPP target

When an authorized native macOS ARM64 build exposes `GameAssembly.dylib`, attach
locally instead of using `-U`:

```sh
frida -f /Applications/AuthorizedGame.app/Contents/MacOS/AuthorizedGame \
  -l tools/runtime/hunter-info-runtime-dump.js \
  > hunter-info-schema-macos.jsonl
```

The script waits for either `libil2cpp.so` or `GameAssembly.dylib`; no object
addresses or platform-specific struct layouts are hardcoded.

## Limitations

- AppGuard, anti-debugging, root detection, integrity checks, or stripped
  IL2CPP exports may prevent attachment or reflection. This guide does not
  include bypasses; use an unobfuscated authorized test build instead.
- A missing export is reported as `hunter-info-schema-error` and must remain an
  unresolved capture gap.
- Duplicate simple class names in different namespaces are emitted separately.
- Method and type tokens are `null` when the relevant optional export is absent.
- Schema evidence cannot prove runtime skill/icon, growth, pet, appearance, or
  save-value bindings by itself.
