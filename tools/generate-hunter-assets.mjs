import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const sourceRoot = path.join(root, "game-assets/extracted/exported");
const assetIndexPath = path.join(root, "apps/web/public/asset-index.json");
const skeletonPath = path.join(sourceRoot, "text/hunter.json__245.bin");
const packageCatalogPath = path.join(root, "packages/content/releases/evil-hunter-1.411/hunter-assets.json");
const publicRoot = path.join(root, "apps/web/public/content/releases/evil-hunter-1.411/hunter-assets");
const publicCatalogPath = path.join(publicRoot, "catalog.json");

const assetIndexPayload = await fs.readFile(assetIndexPath);
const assetIndex = JSON.parse(assetIndexPayload.toString("utf8"));
const skeletonPayload = await fs.readFile(skeletonPath);
const skeleton = JSON.parse(skeletonPayload.toString("utf8"));
const indexedAssets = new Map(assetIndex.assets.map((asset) => [asset.path, asset]));

const evidence = (locator, note) => ({
  sourceId: "evil-hunter-1.411-asset-export",
  locator,
  method: "asset-index-and-source-payload",
  confidence: "confirmed",
  note,
});

function indexed(pattern) {
  return assetIndex.assets.filter((asset) => pattern.test(asset.path)).sort((left, right) => left.path.localeCompare(right.path, undefined, { numeric: true }));
}

async function packageAsset(asset, category) {
  const filename = path.basename(asset.path);
  const output = path.join(publicRoot, category, filename);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.copyFile(path.join(sourceRoot, asset.path), output);
  return {
    id: `${category}:${filename.replace(/__\d+\.png$/, "")}`,
    sourcePath: `game-assets/extracted/exported/${asset.path}`,
    publicPath: `/content/releases/evil-hunter-1.411/hunter-assets/${category}/${filename}`,
    bytes: asset.bytes,
    sha256: asset.sha256,
    evidence: evidence(`asset-index:${asset.path}`, "The exported file path and digest are exact; gameplay semantics are not inferred from the filename."),
  };
}

const portraits = [];
for (const asset of indexed(/^sprites\/hunter_([fm])_(\d+)__[^/]+\.png$/)) {
  const match = asset.path.match(/^sprites\/hunter_([fm])_(\d+)__/);
  const packaged = await packageAsset(asset, "portraits");
  portraits.push({
    ...packaged,
    family: match[1] === "f" ? "female" : "male",
    index: Number(match[2]),
    semanticBinding: "unresolved",
  });
}

const uiFamilies = [
  ["traits", /^sprites\/job_trait_[^/]+\.png$/],
  ["equipment-hud", /^sprites\/ic_hunter_(?:gear_\d+|gear_search|lock)__[^/]+\.png$/],
  ["skills", /^sprites\/(?:skill_h[1-5]_\d+|sub_skill_[^/]+|rp_skill_[^/]+)__[^/]+\.png$/],
  ["hunter-hud", /^sprites\/(?:hp_(?:bg|in|flag|lv_bg_9)|hunter_(?:area_bg|check_[123]|shadow)|assign_hunter_(?:info_box|photo_box|photo_frame)|rp_hunter_(?:box|box_corver|empty)|apvp_(?:myhunter|otherhunter)_frame|fp_hunter_frame|character_(?:info|bar_dummy|graph[0-5]|star_(?:off|on)))__[^/]+\.png$/],
  ["hunter-info-status", /^sprites\/(?:h_detail_ic_\d+|h_detail_stone_box|popup_stat_box_9|stat_frame_9)__[^/]+\.png$/],
  ["hunter-info-tabs", /^sprites\/cha_tab_(?:line_9|line_small_9|off_9|on_9)__[^/]+\.png$/],
  ["hunter-info-equipment", /^sprites\/(?:box_(?:gear_9|item_in_hunter)|equip_(?:bg_9|dummy_(?:0|0[1-8])(?:_on)?|gold_bg_9|sel_bg_9|sel_ic_9))__[^/]+\.png$/],
  ["hunter-info-experience", /^sprites\/exp_gauge_(?:back_9|in_9|in_9_2)__[^/]+\.png$/],
  ["hunter-info-growth", /^sprites\/(?:growth_(?:back_box|btn_(?:dim|off|on)|ic_\d+|in_box|point_bg|top_bg|top_line)|img_growth)__[^/]+\.png$/],
  ["hunter-info-riding-pet", /^sprites\/(?:ride_pet_pasture|rp_(?:cha_box|info_bg_gradian|list_(?:box_(?:deco|frame)|frame)|trait_(?:bg|frame)))__[^/]+\.png$/],
];
const uiAssets = {};
for (const [category, pattern] of uiFamilies) {
  uiAssets[category] = [];
  for (const asset of indexed(pattern)) {
    uiAssets[category].push({
      ...await packageAsset(asset, `ui/${category}`),
      sourceName: path.basename(asset.path).replace(/__\d+\.png$/, ""),
      semanticBinding: "unresolved",
    });
  }
}

const spineSources = [
  ["skeleton", "text/hunter.json__245.bin", "hunter.json"],
  ["atlas", "text/hunter.atlas__258.bin", "hunter.atlas"],
  ["texture", "textures/hunter__166.png", "hunter.png"],
];
const spineBundle = [];
for (const [role, sourcePath, filename] of spineSources) {
  const asset = indexedAssets.get(sourcePath);
  if (!asset) throw new Error(`Missing Hunter Spine asset index entry: ${sourcePath}`);
  const output = path.join(publicRoot, "spine", filename);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.copyFile(path.join(sourceRoot, sourcePath), output);
  spineBundle.push({
    role,
    sourcePath: `game-assets/extracted/exported/${sourcePath}`,
    publicPath: `/content/releases/evil-hunter-1.411/hunter-assets/spine/${filename}`,
    bytes: asset.bytes,
    sha256: asset.sha256,
    evidence: evidence(`sharedassets1.assets:${sourcePath}`, "Atomic Hunter Spine bundle member; all three files are required for rendering."),
  });
}

function hFamily(name) {
  return name.match(/(?:^|_)(h[1-5])(?:_|$)/)?.[1].toUpperCase() ?? "common";
}

const animations = Object.keys(skeleton.animations).sort().map((name) => ({
  name,
  visualFamily: hFamily(name),
  gameplaySemantics: "unresolved",
  evidence: evidence(`hunter.json.animations[${JSON.stringify(name)}]`, "Animation name exists in the confirmed Spine skeleton; timing and gameplay events are not inferred."),
}));
const skinNames = skeleton.skins.map((skin) => skin.name);
const aggregateSkins = skinNames.filter((name) => /^All_h[1-5](?:_|$)/.test(name)).sort().map((name) => ({
  name,
  visualFamily: hFamily(name),
  compositionBinding: "unresolved",
  evidence: evidence(`hunter.json.skins[${JSON.stringify(name)}]`, "Aggregate skin exists and is renderable; it is not bound to a job, Hunter, portrait, or progression row."),
}));
const weaponSkins = skinNames.filter((name) => /^weapon_h[1-5]/.test(name)).sort().map((name) => ({
  name,
  visualFamily: hFamily(name),
  equipmentBinding: "unresolved",
  evidence: evidence(`hunter.json.skins[${JSON.stringify(name)}]`, "Weapon skin exists in the Hunter Spine bundle; no gear content ID mapping is asserted."),
}));

const catalog = {
  schemaVersion: 1,
  catalogId: "evil-hunter-1.411-hunter-assets-v1",
  source: {
    assetIndexPath: "apps/web/public/asset-index.json",
    assetIndexSha256: createHash("sha256").update(assetIndexPayload).digest("hex"),
    miningReportPath: "docs/migration/hunter-mining-report.md",
    policy: "Asset existence and visual-family names are confirmed. Job, class, skill, trait, equipment, and per-Hunter semantic bindings remain unresolved.",
  },
  counts: {
    portraits: portraits.length,
    aggregateSkins: aggregateSkins.length,
    weaponSkins: weaponSkins.length,
    animations: animations.length,
    traits: uiAssets.traits.length,
    equipmentHud: uiAssets["equipment-hud"].length,
    skills: uiAssets.skills.length,
    hunterHud: uiAssets["hunter-hud"].length,
    hunterInfoStatus: uiAssets["hunter-info-status"].length,
    hunterInfoTabs: uiAssets["hunter-info-tabs"].length,
    hunterInfoEquipment: uiAssets["hunter-info-equipment"].length,
    hunterInfoExperience: uiAssets["hunter-info-experience"].length,
    hunterInfoGrowth: uiAssets["hunter-info-growth"].length,
    hunterInfoRidingPet: uiAssets["hunter-info-riding-pet"].length,
  },
  spineBundle,
  portraits,
  visualCatalog: { aggregateSkins, weaponSkins, animations },
  uiAssets,
};

const serialized = `${JSON.stringify(catalog)}\n`;
await fs.mkdir(path.dirname(packageCatalogPath), { recursive: true });
await fs.mkdir(path.dirname(publicCatalogPath), { recursive: true });
await fs.writeFile(packageCatalogPath, serialized);
await fs.writeFile(publicCatalogPath, serialized);
console.log(`Generated Hunter catalog: ${portraits.length} portraits, ${aggregateSkins.length} aggregate skins, ${weaponSkins.length} weapon skins, ${animations.length} animations, ${Object.values(uiAssets).flat().length} UI assets`);
