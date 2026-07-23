# A-Z Migration Status

This is the current implementation handoff for the ordered migration. “Done” means the phase's code/data contract and basic build gate are present; it does not mean every original gameplay rule has been recovered.

| Phase | Status | Delivered | Explicit boundary |
| --- | --- | --- | --- |
| A | Done | APK/XAPK integrity, IL2CPP/C# domain audit, source inventory | Decompiled IL2CPP is evidence, not original source. |
| B | Done | 23,286-object `level1` scene graph, canvas/top/bottom topology, asset/UI/domain catalog | UnityEvent callbacks, sprite PPtrs, runtime mutations and exact screenshots remain open. |
| C | Done | Evidence-aware `original-flow-v1` schema/release, 20 selected assets, 12 unresolved blockers | Release gate remains `runnable=false` until binding evidence is resolved. |
| D | Done | Original intro assets, map candidate, top resources, five BottomView branches, roster shell | `map_new01` is a candidate; exact initial village binding is not claimed. |
| E | Done | Protocol v4 screen state, server-authoritative intents, reconnect/resync, explicit field blockers | No original field monster, combat cadence, damage, drop, or revival rule is claimed. |
| F | Done | Progression/equipment/quest/shop/mail/ads/topup intent contracts and no-grant blockers | No catalog prices, rates, rewards, starter stats, or receipt semantics are guessed. |
| G | Done | 9,359 checksum-pinned runtime-addressable exports, 53/53 Spine families, 116 audio, 2 fonts | Runtime addressability is not behavior/screen binding. |
| H | Done | HttpOnly session bootstrap, Redis session/rate/lease/fencing, PostgreSQL revision guard, migration 0003 | Local educational identity flow; production auth provider and operational secrets remain out of scope. |
| I | Done | Rust fmt/test/clippy, web test/build/audit, asset validators, Docker server/web image builds | Only basic build checks were requested; no final gameplay/load/visual regression gate was run. |
| J | Done | This report and linked evidence/known-gap register | Final original-game compatibility still requires runtime observation and content binding. |

## Current coverage

- Raw source copy: 415/415 files byte-for-byte verified.
- Export catalog: 9,359/9,359 files checksum validated (190,429,626 bytes).
- Spine atomic families: 53/53 complete skeleton + atlas + pages.
- Original-flow published evidence assets: 20; unresolved binding blockers: 12.
- Rust tests: 19 passing; web tests: 10 passing; asset tests: 7 passing.
- Basic production builds: Rust server image and Nginx web image pass.

## What is not yet “full game”

The repository is not yet a 100%-compatible Evil Hunter Tycoon recreation. The remaining work is primarily evidence acquisition and binding: boot/login trace, exact village sprite/layout references, building anchors and interactions, starter hunter composition/stats, first field actor/map, combat formulas, localization tables, mode-specific adventure/raid/PvP state, and provider-backed account/economy semantics. The implementation now fails closed on those unknowns instead of substituting the old fixture.
