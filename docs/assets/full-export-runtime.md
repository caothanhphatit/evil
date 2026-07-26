# Full Export Runtime Catalog

## Contract

The immutable exported evidence set is exposed to the local Docker web origin without copying it into the application source or a generated release directory.

| Route | Purpose |
| --- | --- |
| `/full-assets/manifest.json` | Small bootstrap containing the selected catalog URL, byte length, and SHA-256 |
| `/full-assets/releases/evil-hunter-1.411-export-v1.json` | Versioned catalog for every exported file and all 53 atomic Spine families |
| `/game-assets/<catalog path>` | Read-only asset payload rooted at `game-assets/extracted/exported` |

Consumers must verify the catalog payload against the bootstrap SHA-256 before trusting entries. Each catalog entry then supplies the exact byte length and SHA-256 for its asset. Build a URL by URL-encoding every path segment and appending the catalog path to `/game-assets/`.

The catalog is an evidence transport, not a gameplay release. Its default binding state is `unbound-evidence`; assets become behavior-bound only through a separate validated content release with scene, UI, animation, audio, or server-content evidence.

Versioned village intermediates live under `game-assets/normalized/village`, outside this immutable export root. Their manifest records the Unity bundle, path ID, byte length, and checksum; the current transparent-matte conversion is an explicit approximation that still requires visual validation. Full-catalog validation is exhaustive: any file added below `game-assets/extracted/exported` without a matching index entry fails validation.

## Generate and validate

```bash
pnpm assets:index
pnpm assets:catalog:full
pnpm assets:validate:full
pnpm assets:verify
```

`assets:verify` extracts `base_assets.apk` from the repository XAPK into a temporary directory and verifies all 415 Unity asset files byte-for-byte. Use `tools/verify-asset-copy.sh --source <assets-dir>` only when checking an already extracted authorized source.

`pnpm assets:validate:visible-world` validates the development-only visible-world release separately. It requires exact source and publication hashes, all seven atomic Spine bundles, normalized-village provenance, exhaustive public paths, and explicit unresolved metadata for fixture skins, spawns, map identity, and monster candidates.

Validation hashes all 9,359 files, checks the exact audio/font/metadata/sprite/text/texture counts, validates the bootstrap checksum, and requires skeleton JSON, atlas text, and every referenced page for all 53 Spine families. It also requires explicit outcomes for the four original exporter errors: two recovered fonts and two excluded 0x0 `Font Texture` placeholders.

## Docker mounts

The development `web` service mounts the export read-only at `/workspace/apps/web/public/game-assets`, which makes the payload visible to Vite at `/game-assets/`. The catalog files live under `apps/web/public/full-assets` and are served by the same origin.

The production Nginx configuration maps `/game-assets/` to `/game-assets/`. Run the built image with the same evidence directory mounted read-only:

```bash
docker run --rm -p 8081:80 \
  -v "$PWD/game-assets/extracted/exported:/game-assets:ro" \
  evil-hunter-web
```

Never bake the proprietary evidence set into the web image, upload it to a public registry, or deploy it to a public CDN without distribution rights.
