import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const sourcePath = path.join(repoRoot, "reverse-engineering/evidence/core-economy-tables-v1.json");
const outputPath = path.join(repoRoot, "packages/content/releases/evil-hunter-1.411/gear-catalog.json");
const assetIndexPath = path.join(repoRoot, "apps/web/public/asset-index.json");
const materialIconOutput = path.join(repoRoot, "apps/web/public/content/releases/evil-hunter-1.411/material-icons");
const sourcePayload = await fs.readFile(sourcePath);
const source = JSON.parse(sourcePayload.toString("utf8"));
const assetIndex = JSON.parse(await fs.readFile(assetIndexPath, "utf8"));

const families = [
  ["weapon", "gearWeapons"],
  ["armor", "gearArmor"],
  ["gloves", "gearGloves"],
  ["boots", "gearBoots"],
  ["ring", "gearRing"],
  ["necklace", "gearNecklace"],
  ["belt", "gearBelt"],
];
const materials = Object.fromEntries(source.materials.map((row) => [
  `material:${row.index}`,
  row.localizedNames.en ?? `material:${row.index}`,
]));
const sourceMaterialIcons = new Map(assetIndex.assets.flatMap((asset) => {
  const match = asset.path.match(/^sprites\/shop_product_(\d+)__[^/]+\.png$/);
  return match ? [[Number(match[1]), asset.path]] : [];
}));
const rows = families.flatMap(([kind, key]) => source[key].map((row) => ({
  kind,
  index: row.index,
  job: row.job,
  group: row.group,
  itemLevel: row.itemLevel,
  visible: row.visible,
  sortGroup: row.sortGroup ?? 0,
  name: row.localized.en.title,
  description: row.localized.en.description,
  iconPath: kind === "weapon" && row.group > 2
    ? null
    : `/content/releases/evil-hunter-1.411/gear-icons/${kind}-${row.index}.png`,
  prices: row.buyMoneyByRating,
  materialsByRating: row.craftingMaterialsByRating.map((rating) => rating.ids.map((id, index) => ({
    id: `material:${id}`,
    quantity: rating.quantities[index],
  }))),
})));
const usedMaterialIds = new Set(rows.flatMap((row) => row.materialsByRating.flatMap((rating) => rating.map((cost) => Number(cost.id.split(":")[1])))));
const materialIcons = {};
await fs.mkdir(materialIconOutput, { recursive: true });
for (const materialId of [...usedMaterialIds].sort((left, right) => left - right)) {
  const sourceAsset = sourceMaterialIcons.get(materialId);
  if (!sourceAsset) continue;
  const filename = `material-${materialId}.png`;
  await fs.copyFile(path.join(repoRoot, assetIndex.root, sourceAsset), path.join(materialIconOutput, filename));
  materialIcons[`material:${materialId}`] = `/content/releases/evil-hunter-1.411/material-icons/${filename}`;
}

const catalog = {
  schemaVersion: 1,
  source: {
    path: "reverse-engineering/evidence/core-economy-tables-v1.json",
    sha256: createHash("sha256").update(sourcePayload).digest("hex"),
  },
  difficultyGroups: ["Junk", "Easy", "Normal", "Hard", "Expert", "Nightmare", "Torment"],
  qualityTiers: ["Regular", "Sturdy", "Refined", "Powerful", "Supreme"],
  materials,
  materialIcons,
  rows,
};

await fs.mkdir(path.dirname(outputPath), { recursive: true });
await fs.writeFile(outputPath, `${JSON.stringify(catalog)}\n`);
console.log(`Generated ${rows.length} base gear rows and ${Object.keys(materialIcons).length} material icons at ${path.relative(repoRoot, outputPath)}`);
