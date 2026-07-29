# Generation Rules

## Batch Contract

Every generation batch must record:

- asset family and brief version;
- evidence label and evidence references;
- generator/model and version;
- full positive and negative prompts;
- seed or reproducibility identifier when available;
- generation size, aspect ratio, and any control/reference inputs;
- edit, cleanup, crop, and resize steps;
- author/date and review status.

If the tool cannot expose a seed, record `seed: unavailable` rather than
inventing one.

## Prompt Recipe

Write prompts in this stable order:

1. asset type and gameplay purpose;
2. subject identity and required features;
3. silhouette and proportions;
4. camera/projection and composition;
5. material and surface treatment;
6. palette, lighting, and value structure;
7. rendering language and detail density;
8. background/alpha requirement;
9. technical constraints and forbidden elements.

Do not rely on artist names or copyrighted franchise names as shorthand. Spell
out the observable visual qualities required by the approved references.

## Stable vs Variable Fields

Keep stable across a family:

- camera and projection;
- subject scale and margins;
- outline/render treatment;
- lighting direction;
- palette limits;
- output dimensions;
- negative prompt and cleanup rules.

Change only the fields named as variables in the brief, such as species,
equipment shape, material, pose, or tier accent.

## File Naming

Use lowercase ASCII kebab-case:

```text
<family>--<asset-id>--<variant>--v<NN>.<ext>
```

Examples:

```text
material-icon--iron-ore--base--v01.png
monster-portrait--mon-a-01-1--idle--v03.png
```

The name is a working identifier, not proof of an original-game mapping.

## Review States

- `briefing`: requirements or evidence are incomplete;
- `generating`: batch metadata is complete and generation is active;
- `review`: candidates are ready for comparison;
- `changes-requested`: specific corrections are documented;
- `approved-design`: visual master accepted, not yet runtime-ready;
- `runtime-ready`: manifest, provenance, technical checks, and derivatives pass;
- `blocked-unresolved`: evidence is insufficient to proceed safely.

Only `runtime-ready` assets may be proposed for `game-assets/normalized` or a
web content release.
