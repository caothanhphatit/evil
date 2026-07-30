import { Skin } from "@esotericsoftware/spine-core";
import type { Spine } from "@esotericsoftware/spine-pixi-v8";

interface WeaponPresentation {
  skin: string;
  slot: string;
  attachment: string;
}

// Confirmed packaged base-family skins; these are not gear-index bindings.
const BASE_WEAPON_BY_FAMILY: Record<string, WeaponPresentation> = {
  H1: { skin: "weapon_h1_a_01", slot: "weapon_01", attachment: "sword" },
  H2: { skin: "weapon_h2_a_01", slot: "weapon_02", attachment: "hammer" },
  H3: { skin: "weapon_h3_a_01", slot: "weapon_03", attachment: "bow" },
  H4: { skin: "weapon_h4_a_01", slot: "weapon_04", attachment: "wand" },
  H5: { skin: "weapon_h5_a_01", slot: "weapon_05", attachment: "spear" },
};

export function hunterBaseWeaponSkin(classFamily: string | null): string | null {
  return weaponPresentation(classFamily)?.skin ?? null;
}

export function hunterWeaponAttachment(classFamily: string | null): { slot: string; attachment: string } | null {
  const weapon = weaponPresentation(classFamily);
  return weapon ? { slot: weapon.slot, attachment: weapon.attachment } : null;
}

export function applyHunterSpineSkin(
  spine: Spine,
  skinNames: string[],
  classFamily: string | null,
  compositionName: string,
): void {
  if (skinNames.length === 1 && spine.skeleton.data.findSkin(skinNames[0])) {
    spine.skeleton.setSkinByName(skinNames[0]);
  } else {
    const composition = new Skin(compositionName);
    for (const name of skinNames) {
      const skin = spine.skeleton.data.findSkin(name);
      if (skin) composition.addSkin(skin);
    }
    spine.skeleton.setSkin(composition);
  }
  spine.skeleton.setSlotsToSetupPose();

  const weapon = weaponPresentation(classFamily);
  const familyPrefix = classFamily ? `weapon_${classFamily.toLowerCase()}_` : null;
  if (weapon && familyPrefix && skinNames.some((skinName) => skinName.startsWith(familyPrefix))) {
    spine.skeleton.setAttachment(weapon.slot, weapon.attachment);
  }
}

function weaponPresentation(classFamily: string | null): WeaponPresentation | null {
  return classFamily ? BASE_WEAPON_BY_FAMILY[classFamily.toUpperCase()] ?? null : null;
}
