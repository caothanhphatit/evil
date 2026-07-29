use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(dead_code)]
pub const WEB_REBUILD_GEAR_ROLL_RULESET: &str = "web-rebuild-v1-gear-roll";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftedGearRow {
    pub gear_instance_id: Uuid,
    pub product_id: String,
    pub kind: String,
    pub rating: u16,
    pub quality: u8,
    pub primary_stat: u32,
    pub option_type: u8,
    pub option_value: u16,
    pub icon_path: String,
    pub ruleset: String,
}

#[allow(dead_code)]
pub fn roll_crafted_gear(
    command_id: Uuid,
    row_index: u32,
    product_id: &str,
    kind: &str,
    rating: u16,
) -> CraftedGearRow {
    let mut seed = command_id.as_u128() ^ u128::from(row_index);
    for byte in product_id.bytes().chain(kind.bytes()) {
        seed = seed.rotate_left(7) ^ u128::from(byte);
        seed = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    let next = |value: &mut u128| {
        *value ^= *value << 13;
        *value ^= *value >> 7;
        *value ^= *value << 17;
        *value
    };
    let quality = (next(&mut seed) % 5) as u8;
    let rating_scale = u32::from(rating).saturating_add(1);
    let primary_stat = 10_u32
        .saturating_mul(rating_scale)
        .saturating_add((next(&mut seed) % u128::from(10 * rating_scale)) as u32);
    let option_type = (next(&mut seed) % 8) as u8;
    let option_value = (1 + next(&mut seed) % u128::from(5 * rating_scale)) as u16;
    let gear_instance_id = Uuid::from_u128(
        command_id.as_u128() ^ (u128::from(row_index).wrapping_add(1) << 64) ^ seed,
    );
    let catalog_index = product_id
        .split(':')
        .nth(2)
        .and_then(|value| value.parse::<u32>().ok())
        .expect("validated gear recipe contains a catalog index");
    CraftedGearRow {
        gear_instance_id,
        product_id: product_id.to_owned(),
        kind: kind.to_owned(),
        rating,
        quality,
        primary_stat,
        option_type,
        option_value,
        icon_path: format!(
            "/content/releases/evil-hunter-1.411/gear-icons/{kind}-{catalog_index}.png"
        ),
        ruleset: WEB_REBUILD_GEAR_ROLL_RULESET.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_roll_is_deterministic_and_row_specific() {
        let command = Uuid::from_u128(42);
        let first = roll_crafted_gear(command, 0, "recipe:weapon:0:rating:1", "weapon", 1);
        assert_eq!(
            first,
            roll_crafted_gear(command, 0, &first.product_id, "weapon", 1)
        );
        assert_ne!(
            first,
            roll_crafted_gear(command, 1, &first.product_id, "weapon", 1)
        );
        assert_eq!(first.ruleset, WEB_REBUILD_GEAR_ROLL_RULESET);
        assert_eq!(
            first.icon_path,
            "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png"
        );
    }
}
