# Hunter Source-Recovery Boundary v1

Date: 2026-07-26
Package: Evil Hunter Tycoon `1.411`

## Findings

The supplied XAPK contains `config.arm64_v8a.apk`, which contains the stripped
ARM64 `libil2cpp.so` (`104,557,288` bytes). The package also supplies
`global-metadata.dat`. There is no recoverable `Assembly-CSharp.dll` in the
package, so the original managed C# source is not present as a normal assembly.

The current repository keeps the metadata and decoded evidence, but does not
commit a second copy of the 100 MB native binary. The binary can be extracted to
a temporary analysis directory from the XAPK when needed.

## What can be reconstructed

- Unity serialized QuickSheet rows and local Addressables content.
- IL2CPP type/field/method boundaries when metadata and runtime reflection are
  available.
- Method signatures, tokens, field offsets and class relationships from the
  controlled API35/API30 schema captures.
- Runtime state transitions and formulas by authorized before/after capture.
- Partial native control flow if a matching unprotected binary, symbols or a
  stable runtime trace becomes available.

## What cannot be claimed from the current APK alone

- Original C# files, local variable names or comments.
- Exact protected method bodies for Hunter generation, damage, loot, gear rolls,
  skill execution, pet effects or save encoding.
- A semantic mapping from a field/method name to a gameplay rule based only on
  string resemblance.

The current ARM64 binary is stripped and ordinary string extraction exposes no
usable first-party Hunter identifiers. Existing evidence therefore uses
serialized tables, metadata correlation and reflection/runtime boundaries rather
than pretending to be a decompiled source tree.

## ADB dump value

An authorized old ADB dump can add value if it contains:

- private app save files captured with root/debuggable backup access;
- runtime-created Addressables/cache files not present in the APK;
- before/after state files around one controlled Hunter action;
- matching `libil2cpp.so` and metadata from the same package build.

External storage alone is not enough for private save data. The API35 inventory
already showed that its runtime `global-metadata.dat` is byte-identical to the
package copy and added only `unity.ver` plus opaque shader-cache files.

## Safe next step

Use a disposable authorized account and a stable ARM64 session to capture one
action at a time: skill study, gear equip/enhance, trait/growth unlock, pet
assignment and farm/sell. Preserve package version, device ABI, timestamps,
exact action and before/after snapshots. These traces are the practical route to
recovering mechanics that static C# reconstruction cannot provide.
