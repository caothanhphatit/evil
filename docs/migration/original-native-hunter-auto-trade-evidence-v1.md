# Original Hunter Auto-Trade Native Evidence V1

Date: 2026-07-28
Package: Evil Hunter Tycoon `1.411` (`26071501`)

## Capture boundary

The reviewed methods were captured from an authorized API 35 ARM64 session by
the existing one-shot IL2CPP native-method dumper. Frida detached immediately
after the record; no managed object values, account identifiers, or raw saves
were read. The external `/proc/PID/mem` reader was used to verify that the same
module offsets are present, but those pages are protected/encrypted when the
methods have not been live-decrypted.

Evidence: `reverse-engineering/evidence/original-native-hunter-auto-trade-decrypted-api35-v1.json`.

## Confirmed method set

| Method | Token | Module offset | Native bytes |
| --- | ---: | ---: | ---: |
| `ItemPotionBuy` | `100686720` | `0x341a7f0` | `0x20c` |
| `SpeakItemSell` | `100686728` | `0x341c404` | `0x70` |
| `SpeakWeaponBuy` | `100686749` | `0x34240b0` | `0x1b8` |
| `SpeakArmorBuy` | `100686996` | `0x3424268` | `0x74` |
| `ItemArmorBuy` | `100687026` | `0x3469de0` | `0x224` |
| `ItemWeaponBuy` | `100687040` | `0x346ae90` | `0x224` |
| `ItemAccessoryBuy` | `100687052` | `0x346b520` | `0x224` |

## Shared structural facts

The three gear-buy bodies (`ItemArmorBuy`, `ItemWeaponBuy`, and
`ItemAccessoryBuy`) have the same native size and the same high-level control
flow shape:

1. Initialize a shared static/runtime block once.
2. Call a Hunter-local helper with literal selector `15` and literal mode `1`.
3. Resolve a candidate item and branch to the common failure return when the
   candidate or required shared state is null.
4. Clear a two-byte field at Hunter offset `0x1f4` before building the purchase
   result.
5. Write a 16-byte value at Hunter offset `0x1b4` and a 32-bit value at offset
   `0x1c4`.
6. Call a helper using the pending object at Hunter offset `0xa80`, then clear
   that pending-object field.
7. Invoke four repeated mutation helpers with values loaded from a shared
   catalog/runtime object; return the helper's Boolean success bit.

The gear variants differ in helper targets and catalog offsets. The native
body alone does not prove whether those helpers debit Hunter money, create an
owned instance, equip it, or update town stock.

`ItemPotionBuy` follows the same pending-object cleanup and result-write shape,
but has three repeated mutation-helper calls after the pending object is
cleared. `SpeakItemSell`, `SpeakWeaponBuy`, and `SpeakArmorBuy` are shorter UI/
conversation-side methods and read Hunter field offset `0x200` in the
captured body. Their presentation role is not a transaction proof.

## Integration boundary

This pass does not connect the methods to Rust economy state. The following
identities remain unresolved and must stay fail-closed:

- buyer-selection inputs and desired-upgrade comparison;
- Hunter wallet debit and town credit sinks;
- shop stock decrement and product ownership creation;
- old-equipment transfer/equip semantics;
- material seller attribution and partial-sale behavior;
- meanings of offsets `0x1b4`, `0x1c4`, `0x1f4`, `0x200`, and `0xa80`.

The captured method bodies are therefore reference evidence only. A live Rust
implementation may integrate these flows only after typed before/after value
captures or independently resolved helper bodies bind each mutation.

## Direct-call resolution pass

The complete `Assembly-CSharp` method index was captured in a separate one-shot
pass and compared with every direct `BL` target in the seven bodies. Several
targets resolve to shared/internal thunks or generic method bodies, while other
targets land inside a larger native range rather than at a stable managed method
entry. No target can therefore be named as a wallet debit, stock decrement,
ownership insertion, or equip mutation from module offset alone. Those helper
identities remain unresolved by policy.
