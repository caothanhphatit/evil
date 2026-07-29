# Asset Studio

This directory is the shared design workspace for rebuilding and extending the
game's visual assets. It is intentionally separate from `game-assets/`, which
contains migration source, normalized runtime assets, and manifests.

## Purpose

- keep generation references, briefs, prompts, and reviews together;
- make batches reproducible across tools and sessions;
- distinguish recovered game evidence from new rebuild artwork;
- prevent drafts from entering the runtime asset pipeline by accident.

## Workflow

1. Create an asset brief from `templates/asset-brief.md`.
2. Add approved evidence or visual references under `references/<family>/`.
3. Lock the family rules in the brief before generating a batch.
4. Save prompts and generation metadata under `prompts/<family>/`.
5. Put unreviewed outputs under `work-in-progress/<family>/`.
6. Review the batch with `REVIEW-CHECKLIST.md`.
7. Move accepted masters to `approved/<family>/` with their metadata.
8. Promote an accepted asset into the runtime pipeline only through the
   repository's asset manifest and deterministic transformation workflow.

## Required Reading

- `docs/KT.md`
- `docs/assets/asset-migration-spec.md`
- `STYLE-GUIDE.md`
- `GENERATION-RULES.md`

## Evidence Labels

Every brief and approved asset must use exactly one label:

- `migrated-exact`: pixels or data directly recovered from an approved source;
- `reference-reconstructed`: recreated from cited screenshots or package evidence;
- `rebuild-original`: newly designed for this clean-room web rebuild;
- `unresolved`: insufficient evidence; generation is blocked, not guessed.

Generated artwork must never be labeled `migrated-exact`.
