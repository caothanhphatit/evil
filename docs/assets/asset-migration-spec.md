# Asset Migration Specification

## Objective

Copy and make runnable every legally approved source asset required by the game: textures, sprites, sprite atlases, maps, tiles, prefabs as data, animation clips/controllers, skeleton data, materials, shaders or replacements, particles, fonts, localization, audio, video, configuration tables, and their dependencies.

“100% copied” means every approved source object is represented in the manifest as migrated, intentionally transformed, replaced with an approved equivalent, or explicitly excluded with a reason. “100% runnable” additionally means all references resolve and automated plus human validation passes in supported browsers.

## Directory Model

```text
assets/
  source/       immutable extracted evidence; never served directly
  normalized/   canonical lossless/intermediate forms
  manifests/    IDs, provenance, dependency graph, checksums, status
  generated/    reproducible browser-ready output; not hand-edited
```

If the repository chooses different physical paths, preserve these logical stages.

## Required Manifest Fields

Each source object has:

- stable canonical asset ID;
- source package/version, bundle/path, Unity object ID/type;
- source and normalized SHA-256;
- media metadata: dimensions, format, color space, sample rate, duration, frame count;
- dependencies and reverse references;
- transformation pipeline/tool version;
- output paths and content hashes;
- license/provenance review state;
- migration state and validation state;
- notes for unsupported or approximate behavior.

IDs are stable across builds. Browser filenames may be content-addressed but never become business identifiers.

## Asset Classes

### Sprites and textures

- Preserve original pixel dimensions, pivots, borders, slicing, packing tags, filtering, wrap mode, transparency, and pixels-per-unit semantics.
- Generate atlases by loading locality and render layer, not arbitrary maximum size.
- Extrude atlas edges to prevent bleeding; verify nearest/linear filtering per asset.
- Retain a lossless normalized master. Generate WebP/AVIF/PNG and GPU-compressed variants only through reproducible builds.

### Animation

- Preserve clip names, frame order, timestamps, loop mode, speed, events, transitions, layers, blend parameters, pivots, and sprite/skeleton references.
- Convert Unity controllers into a canonical state graph interpreted by the web client.
- Gameplay events such as hit timing remain server rules; animation events are presentation cues and cannot award damage.
- Validate clips through contact sheets and captured playback comparison.

### Maps, scenes, and prefabs

- Convert scene/prefab objects into versioned declarative schemas: transforms, hierarchy, render order, collision/navigation regions, spawn markers, interaction points, and asset references.
- Strip executable Unity behavior. Reimplement behavior in typed client/server systems.
- Preserve unknown serialized fields in evidence, not silently drop them.

### Audio and video

- Preserve lossless source where available, loop points, volume, pitch range, mixer group intent, spatial mode, and event mapping.
- Generate supported delivery variants and account for browser autoplay restrictions.
- Compare duration and loop discontinuity; normalize loudness only through an approved rule.

### Fonts and localization

- Preserve font files, fallback chains, glyph coverage, line metrics, rich-text conventions, locale, plural rules, and key IDs.
- Subset only after automated glyph coverage across every shipped locale.
- No user-facing text is hard-coded when a source key exists.

### Shaders, materials, and effects

- Inventory render intent: blend mode, masks, tint, distortion, dissolve, outlines, particles, trails, and timing.
- Port to PixiJS filters/meshes or document an approved visual equivalent.
- Provide device-tier fallbacks for costly effects without changing authoritative outcomes.

### Content and configuration

- Treat tables/rates as versioned game content, not untyped assets.
- Import to canonical schemas with exact numeric representation and cross-reference validation.
- Server uses authoritative releases; client receives only what presentation needs.

## Pipeline Requirements

1. Inventory immutable source and calculate checksums.
2. Extract object metadata and dependency references.
3. Normalize losslessly into canonical formats.
4. Validate dimensions, duration, IDs, references, and expected counts.
5. Generate optimized browser variants and atlases.
6. Publish an immutable, versioned, content-addressed manifest.
7. Load-test, visually compare, and archive reports.

Every transformation is scripted, pinned, deterministic, and runnable in Docker/CI. Hand repair occurs in normalized source with documented provenance, never inside generated output.

## Runtime Loading

- Bootstrap manifest contains only the boot screen and metadata needed to select a content release.
- Bundles align with vertical slices/scenes and are lazy-loaded.
- Critical assets preload; optional content streams in with bounded concurrency and cancellation.
- CDN responses use immutable caching by hash. Service worker and IndexedDB caches are version-aware.
- A missing required asset is a release failure, not silently replaced in production.

## Validation And Coverage

Automated checks include:

- source object count and byte coverage by class;
- duplicate/missing IDs and dangling dependencies;
- decode/load in Chromium, Firefox, and WebKit targets;
- atlas bounds, pivots, frame sequence, duration, loop, glyph, and audio checks;
- memory, upload time, bundle size, and first-scene budgets;
- deterministic manifest reproduction from the same inputs.

Human review includes representative visual diff, animation playback, map layering, effect timing, audio loops, UI nine-slice behavior, and supported-device checks.

## Acceptance Criteria

- 100% of approved sources classified in the manifest.
- 100% of required references resolve for released content.
- All required assets decode and render/play in supported browsers.
- No source checksum changes without a provenance record.
- No generated file is manually edited.
- Release reports list exact exclusions, approximations, and device fallbacks.
- Licensing/provenance review approves distribution before any public deployment.
