import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const sourceRoot = path.join(repoRoot, "game-assets/extracted/exported");
const outputRoot = path.join(repoRoot, "apps/web/public/content/releases/visible-world-v1");
const villageRoot = path.join(repoRoot, "game-assets/normalized/village");
const families = ["hunter", "Chief", "Npc", "npc_animal", "pet", "mon_goldblin", "mon_a_01_1"];
const actorClaims = {
  hunter: { runtimeRole: ["resolved", "confirmed"], note: "Hunter family is confirmed; starter skin, composition, and spawn are unresolved." },
  Chief: { runtimeRole: ["candidate", "strongly-inferred"], note: "Chief identity is strongly inferred; village placement and starter composition are unresolved." },
  Npc: { runtimeRole: ["candidate", "strongly-inferred"], note: "Generic NPC family is strongly inferred; saved-scene skin and placement are unresolved." },
  npc_animal: { runtimeRole: ["candidate", "strongly-inferred"], note: "Animal family is strongly inferred; species, skin, and placement are unresolved." },
  pet: { runtimeRole: ["candidate", "strongly-inferred"], note: "Pet family is strongly inferred; owned/default pet and placement are unresolved." },
  mon_goldblin: { runtimeRole: ["candidate", "tentative"], note: "Render-capable passive candidate only; no original field or combat identity is claimed." },
  mon_a_01_1: { runtimeRole: ["candidate", "tentative"], note: "Render-capable combat candidate only; original first-field identity is unresolved." },
};
const assetIndex = JSON.parse(await fs.readFile(path.join(repoRoot, "game-assets/asset-index.json"), "utf8"));
const byPath = new Map(assetIndex.assets.map((asset) => [asset.path, asset]));

function uniquePath(prefix, suffix) {
  const matches = assetIndex.assets.filter((asset) => asset.path.startsWith(prefix) && asset.path.endsWith(suffix));
  if (matches.length !== 1) throw new Error(`Expected one ${prefix}*${suffix}, found ${matches.length}`);
  return matches[0].path;
}

async function publish(sourcePath, outputPath) {
  const indexed = byPath.get(sourcePath);
  if (!indexed) throw new Error(`Missing indexed source ${sourcePath}`);
  const payload = await fs.readFile(path.join(sourceRoot, sourcePath));
  const digest = createHash("sha256").update(payload).digest("hex");
  if (payload.length !== indexed.bytes || digest !== indexed.sha256) throw new Error(`Checksum mismatch for ${sourcePath}`);
  const output = path.join(outputRoot, outputPath);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.writeFile(output, payload);
  return { sourceNamespace: "immutable-export", sourcePath, publicPath: `/content/releases/visible-world-v1/${outputPath}`, bytes: indexed.bytes, sha256: indexed.sha256 };
}

async function publishLocal(sourcePath, outputPath) {
  const payload = await fs.readFile(path.join(repoRoot, sourcePath));
  const output = path.join(outputRoot, outputPath);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.writeFile(output, payload);
  return { sourceNamespace: "normalized-evidence", sourcePath, publicPath: `/content/releases/visible-world-v1/${outputPath}`, bytes: payload.length, sha256: createHash("sha256").update(payload).digest("hex") };
}

await fs.rm(outputRoot, { recursive: true, force: true });
const actors = [];
for (const family of families) {
  const skeletonSource = uniquePath(`text/${family}.json__`, ".bin");
  const atlasSource = uniquePath(`text/${family}.atlas__`, ".bin");
  const textureSource = uniquePath(`textures/${family}__`, ".png");
  actors.push({
    family,
    runtimeUse: "migration-fixture",
    evidence: {
      sourceBundle: { resolution: "resolved", confidence: "confirmed", basis: "full-asset-catalog" },
      runtimeRole: { resolution: actorClaims[family].runtimeRole[0], confidence: actorClaims[family].runtimeRole[1] },
      skin: { resolution: "unresolved", confidence: "unknown" },
      spawn: { resolution: "unresolved", confidence: "unknown" },
      legacyGameplayIdentity: { resolution: "unresolved", confidence: "unknown" },
      note: actorClaims[family].note,
    },
    skeleton: await publish(skeletonSource, `actors/${family}/${family}.json`),
    atlas: await publish(atlasSource, `actors/${family}/${family}.atlas`),
    texture: await publish(textureSource, `actors/${family}/${family}.png`),
  });
}

const fieldMap = {
  ...await publish("textures/map_new01__185.png", "maps/map_new01.png"),
  runtimeUse: "migration-fixture",
  evidence: {
    resolution: "candidate",
    confidence: "tentative",
    note: "map_new01 is source-confirmed but is not verified as the original first field or a complete village map."
  }
};
const tileBindings = [
  ["background_01__1548.png", 4.30, 14.11, 499], ["background_02__1515.png", 9.42, 14.11, 499],
  ["background_05__1522.png", 24.78, 15.39, 499], ["background_06__1547.png", 29.90, 14.11, 499],
  ["background_07__1533.png", 4.30, 8.99, 499], ["background_08__1530.png", 9.42, 8.99, 499],
  ["background_11__1508.png", 24.78, 7.71, 499], ["background_12__1519.png", 29.90, 8.99, 499],
  ["background_13__1506.png", 4.30, 3.87, 499], ["background_14__1541.png", 9.42, 3.87, 499],
  ["background_15__1542.png", 14.54, 3.87, 499], ["background_16__1517.png", 19.66, 3.87, 499],
  ["background_17__1516.png", 24.78, 3.87, 499], ["background_18__1535.png", 29.90, 3.87, 499],
  ["back_anim_a_01__1529.png", 17.10, 11.55, 494], ["back_anim_b_01__1546.png", 24.78, 11.55, 494],
];
const villageTiles = await Promise.all(tileBindings.map(async ([file, x, y, z]) => ({
  ...await publish(`sprites/${file}`, `village/background/${file}`), x, y, z,
})));
const extracted = JSON.parse(await fs.readFile(path.join(villageRoot, "manifest.json"), "utf8"));
const foregroundBindings = [
  ["ground", 18.01, 10.53, 489], ["gate", 13.06, 14.26, 493], ["wallA", 18.73, 8.55, 486],
  ["wallB", 14.33, 9.53, 488], ["wallC", 22.43, 9.915, 488], ["wallD", 13.93, 11.42, 490],
  ["wallE", 19.515, 12.115, 492], ["bridgeA", 14.86, 12.49, 491], ["bridgeB", 22.17, 9.43, 487], ["bridgeC", 15.30, 8.67, 487],
];
const foreground = await Promise.all(foregroundBindings.map(async ([id, x, y, z]) => ({
  ...await publishLocal(`game-assets/normalized/village/foreground/${id}.png`, `village/foreground/${id}.png`), id, x, y, z,
})));
const npcPositions = { farm_npc_1: [20.839, 12.731, 492], farm_npc_2: [21.96, 13.25, 492], fallen_pasture_npc: [19.624, 13.71, 492] };
const npcs = Object.fromEntries(await Promise.all(Object.entries(extracted.npcs).map(async ([role, frames]) => [role, {
  position: { x: npcPositions[role][0], y: npcPositions[role][1], z: npcPositions[role][2] },
  frames: await Promise.all(frames.map(async (frame) => ({
    ...await publishLocal(`game-assets/normalized/village/npcs/${role}/${frame.name}.png`, `village/npcs/${role}/${frame.name}.png`),
    frame: frame.frame,
    name: frame.name
  }))),
}])));
const decorations = Object.entries(npcs).map(([role, npc]) => ({
  id: role,
  publicPath: npc.frames[0].publicPath,
  x: npc.position.x,
  y: npc.position.y,
  z: npc.position.z,
  frames: npc.frames,
}));
for (const [id, sourcePath, x, y, z] of [
  ["mole_npc_1", "sprites/img_mole_npc_1_0__382.png", 11.55, 10.15, 492],
  ["mole_npc_2", "sprites/img_mole_npc_2_0__856.png", 12.45, 10.15, 492],
  ["rift_village_npc", "sprites/img_devilmotion_01_0__8345.png", 20.28, 6.98, 492],
]) {
  decorations.push({ id, ...(await publish(sourcePath, `village/decorations/${id}.png`)), x, y, z });
}
const village = {
  tiles: villageTiles,
  foreground,
  decorations,
  npcs,
  bindingState: "partial-scene-derived",
  confidence: "confirmed",
  completeness: "partial",
  evidenceReference: "docs/migration/corrective-village-binding-report.md",
  unresolved: ["complete-building-layout", "camera-bounds", "runtime-building-state", "dynamic-actor-spawns"]
};
const manifest = {
  schemaVersion: 3,
  releaseId: "visible-world-v1",
  releaseState: "development-evidence",
  bindingState: "mixed-resolved-and-unresolved",
  evidencePolicy: {
    runtimeAuthority: "presentation-only",
    fixtureLabelRequired: true,
    allowedConfidence: ["confirmed", "strongly-inferred", "tentative", "unknown"],
    note: "Source-complete assets may still have unresolved skin, spawn, role, map, or gameplay bindings."
  },
  runtimeDiagnostics: {
    fixture: true,
    unresolved: ["starter-skins", "dynamic-spawns", "first-field-map", "first-field-monster", "gameplay-rules"]
  },
  map: fieldMap,
  fieldMap,
  village,
  actors
};
const serializedManifest = `${JSON.stringify(manifest, null, 2)}\n`;
await fs.writeFile(path.join(outputRoot, "release.json"), serializedManifest);
const releasePayload = Buffer.from(serializedManifest);
const bootstrap = {
  schemaVersion: 3,
  releaseId: "visible-world-v1",
  releasePath: "/content/releases/visible-world-v1/release.json",
  releaseBytes: releasePayload.length,
  releaseSha256: createHash("sha256").update(releasePayload).digest("hex"),
};
await fs.writeFile(path.join(outputRoot, "manifest.json"), `${JSON.stringify(bootstrap, null, 2)}\n`);
console.log(`Packaged ${actors.length} evidence-safe Spine fixtures and one unresolved field-map candidate.`);
