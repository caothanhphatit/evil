# Normalized Asset Evidence

This directory contains versioned intermediate evidence, not application build output.

`village/manifest.json` pins each file to its immutable Unity bundle and path ID, plus its byte length and SHA-256. The current UnityPy export applies a near-white transparent-matte approximation for browser compositing. That transformation is reproducible, but it is not considered visually source-faithful until reference-image validation passes.

Browser publication is generated separately under `apps/web/public/content/releases/` and must pass the release-specific validator.
