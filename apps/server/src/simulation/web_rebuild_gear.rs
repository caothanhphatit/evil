use std::sync::OnceLock;

use serde::Deserialize;
use uuid::Uuid;

use super::rng::DeterministicRng;

pub const WEAPON_ROLL_RULESET: &str = "evil-hunter-rebuild-v1.weapon-core-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RolledCraftedWeapon {
    pub gear_instance_id: Uuid,
    pub weapon_id: String,
    pub quality: u8,
    pub attack_damage: u32,
    pub icon_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuildWeaponDefinition {
    pub gear_index: u32,
    pub weapon_id: String,
    pub class_id: String,
    pub visual_family: String,
    pub display_name_en: String,
    pub display_name_vi: String,
    pub attack_damage_min: u32,
    pub attack_damage_max: u32,
    pub icon_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeaponCatalog {
    weapons: Vec<WeaponBase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeaponBase {
    id: String,
    class_id: String,
    attack_damage_min: u32,
    attack_damage_max: u32,
    localization: WeaponLocalization,
    visual: WeaponVisual,
}

#[derive(Debug, Deserialize)]
struct WeaponLocalization {
    en: String,
    vi: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeaponVisual {
    family: String,
    expected_inventory_icon: String,
}

fn catalog() -> &'static WeaponCatalog {
    static CATALOG: OnceLock<WeaponCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json"
        ))
        .expect("reviewed rebuild weapon catalog must decode")
    })
}

pub fn roll_crafted_weapon(
    roll_id: Uuid,
    gear_index: u32,
    rating: u16,
) -> Option<RolledCraftedWeapon> {
    let base = catalog_weapon(gear_index)?;
    let roll = roll_id.as_u128();
    let seed =
        (roll as u64) ^ ((roll >> 64) as u64) ^ u64::from(gear_index) ^ (u64::from(rating) << 48);
    let mut rng = DeterministicRng::new(seed);
    let attack_damage = rng.range_inclusive(
        i32::try_from(base.attack_damage_min).ok()?,
        i32::try_from(base.attack_damage_max).ok()?,
    );
    Some(RolledCraftedWeapon {
        gear_instance_id: roll_id,
        weapon_id: base.id.clone(),
        quality: u8::try_from(rating.min(4)).ok()?,
        attack_damage: u32::try_from(attack_damage).ok()?,
        icon_path: base.visual.expected_inventory_icon.clone(),
    })
}

pub fn rebuild_weapon_definition(product_id: &str) -> Option<RebuildWeaponDefinition> {
    let gear_index = parse_weapon_product_id(product_id)?;
    let base = catalog_weapon(gear_index)?;
    Some(RebuildWeaponDefinition {
        gear_index,
        weapon_id: base.id.clone(),
        class_id: base.class_id.clone(),
        visual_family: base.visual.family.clone(),
        display_name_en: base.localization.en.clone(),
        display_name_vi: base.localization.vi.clone(),
        attack_damage_min: base.attack_damage_min,
        attack_damage_max: base.attack_damage_max,
        icon_path: base.visual.expected_inventory_icon.clone(),
    })
}

fn catalog_weapon(gear_index: u32) -> Option<&'static WeaponBase> {
    let icon_filename = format!("weapon-{gear_index}.png");
    catalog().weapons.iter().find(|weapon| {
        weapon
            .visual
            .expected_inventory_icon
            .ends_with(&icon_filename)
    })
}

fn parse_weapon_product_id(product_id: &str) -> Option<u32> {
    let mut parts = product_id.split(':');
    if parts.next()? != "recipe" || parts.next()? != "weapon" {
        return None;
    }
    let gear_index = parts.next()?.parse().ok()?;
    if parts.next()? != "rating" || parts.next()?.parse::<u8>().is_err() || parts.next().is_some() {
        return None;
    }
    Some(gear_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_roll_is_deterministic_and_stays_inside_the_base_range() {
        let first = roll_crafted_weapon(Uuid::from_u128(42), 0, 1).unwrap();
        let replay = roll_crafted_weapon(Uuid::from_u128(42), 0, 1).unwrap();
        assert_eq!(first, replay);
        assert_eq!(first.gear_instance_id, Uuid::from_u128(42));
        assert_eq!(first.weapon_id, "wp_berserker_000");
        assert!((60..=96).contains(&first.attack_damage));
        assert!(first.icon_path.ends_with("weapon-0.png"));
    }

    #[test]
    fn unknown_package_weapon_index_fails_closed() {
        assert!(roll_crafted_weapon(Uuid::from_u128(42), 999, 1).is_none());
    }

    #[test]
    fn purchased_weapon_definition_is_resolved_from_the_recipe_identity() {
        let weapon = rebuild_weapon_definition("recipe:weapon:0:rating:0").unwrap();
        assert_eq!(weapon.weapon_id, "wp_berserker_000");
        assert_eq!(weapon.visual_family, "H1");
        assert_eq!(weapon.display_name_vi, "Đại Kiếm Sắt Mẻ");
        assert!(weapon.icon_path.starts_with("/content/"));
        assert!(rebuild_weapon_definition("recipe:helmet:0:rating:0").is_none());
    }
}
