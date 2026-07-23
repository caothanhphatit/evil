# Web vertical slice

PixiJS client for the server-authoritative rebuild. It connects to `VITE_WORLD_WS_URL`, buffers authoritative snapshots, and renders interpolated frames. When no server is available it automatically starts a local demo simulation.

## Run standalone

```bash
cd apps/web
npm install
npm run dev
```

Build and test with `npm run build` and `npm test`.

## Exported assets

The complete export is discoverable through `/full-assets/manifest.json`. That checksum-pinned bootstrap selects a versioned catalog whose asset paths are relative to `/game-assets/`. Docker mounts the immutable export read-only; it is not copied into the application image.

The full catalog is evidence-only and defaults to `unbound-evidence`. Rendering code must use an approved content release rather than treating an asset filename as proof of its scene, UI, audio, animation, or gameplay role.

The renderer searches the catalog for grave/tomb/cemetery art and uses it when available. Procedural visuals keep the slice runnable before the extraction pipeline finishes.

## Snapshot contract

```json
{
  "sequence": 12,
  "serverTime": 1760000000000,
  "gold": 12480,
  "day": 18,
  "entities": [
    { "id": 1, "kind": "hunter", "x": 320, "y": 360, "hp": 93, "maxHp": 100, "name": "Hunter 1" }
  ]
}
```
