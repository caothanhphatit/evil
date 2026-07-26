# Local Agent Entry Point

Read `docs/KT.md` completely before changing this repository. It contains the
current architecture, verified migration evidence, unresolved gaps, runtime
capture workflow, and validation rules.

Project rules:

- Do not invent original-game mappings, values, icons, mechanics, or fallback
  data. Mark unsupported behavior as unresolved.
- Prefer evidence under `reverse-engineering/evidence/` and reports under
  `docs/migration/` over filename order or visual guesses.
- Keep FE, server contracts, relational migrations, and generated content in
  sync. Reuse the existing building and Hunter Info patterns.
- Preserve user changes and do not commit build output, dependency caches,
  secrets, or newly acquired proprietary inputs outside the existing Git LFS
  policy.
- Use low-resource static tooling first. Follow the authorized runtime capture
  procedure in `docs/migration/hunter-info-runtime-capture-macos.md` when a
  physical Android ARM64 device is available.

