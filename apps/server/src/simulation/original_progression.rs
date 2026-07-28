use std::sync::OnceLock;

use serde::Deserialize;

const EXPERIENCE_CATALOG_JSON: &str = include_str!(
    "../../../../packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json"
);

/// Exact `1.411` cap for stored `HunterData.level`. Product-facing UI displays
/// this value plus one, so the corresponding displayed cap is level 100.
pub const ORIGINAL_HUNTER_MAX_STORED_LEVEL: u32 = 99;
#[allow(dead_code)]
pub const ORIGINAL_HUNTER_MAX_DISPLAY_LEVEL: u32 = ORIGINAL_HUNTER_MAX_STORED_LEVEL + 1;

/// Exact presentation/mission projection used by the recovered `PlusExp` path.
#[allow(dead_code)]
pub fn original_hunter_display_level(stored_level: u32) -> u32 {
    stored_level.wrapping_add(1)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExperienceCatalog {
    rows: Vec<ExperienceRow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExperienceRow {
    index: u32,
    experience_by_difficulty: [u64; 6],
}

fn experience_catalog() -> &'static ExperienceCatalog {
    static CATALOG: OnceLock<ExperienceCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(EXPERIENCE_CATALOG_JSON)
            .expect("generated original experience catalog must remain valid")
    })
}

/// Exact `GameManager.GetNeedExp(revive, currentLevel)` table lookup.
/// Selectors outside `0..=4` take the native exp6/default branch.
#[allow(dead_code)] // Kept disconnected until the original global max-level source is recovered.
pub fn original_need_experience(revive: u32, current_level: u32) -> Option<u64> {
    let row_index = current_level.checked_add(1)?;
    let row = experience_catalog()
        .rows
        .get(usize::try_from(row_index).ok()?)?;
    if row.index != row_index {
        return None;
    }
    let column = if revive <= 4 {
        usize::try_from(revive).ok()?
    } else {
        5
    };
    Some(row.experience_by_difficulty[column])
}

/// Replays the exact `PlusExp` carry rule at the recovered global level cap.
/// Exact-threshold EXP does not level up; carry requires `> 0`.
#[allow(dead_code)]
pub fn original_apply_experience(
    mut level: u32,
    mut current_experience: u64,
    mut incoming_experience: u64,
    revive: u32,
) -> Option<(u32, u64)> {
    if level >= ORIGINAL_HUNTER_MAX_STORED_LEVEL {
        return Some((level, current_experience));
    }

    loop {
        let required = original_need_experience(revive, level)?;
        let missing = required.saturating_sub(current_experience);
        if incoming_experience > missing {
            incoming_experience -= missing;
            level = level.saturating_add(1);
            current_experience = 0;
            if level >= ORIGINAL_HUNTER_MAX_STORED_LEVEL {
                return Some((level, current_experience));
            }
            continue;
        }

        current_experience = current_experience.saturating_add(incoming_experience);
        return Some((level, current_experience));
    }
}

/// Exact selector feeding the separate `mBuildingSoulUp` progression branch in
/// `PlusExp`. Its product-facing name and downstream formula remain unresolved.
#[allow(dead_code)]
pub fn original_secondary_progression_base(
    revive: u32,
    hunter_level: u32,
    stage_level: u32,
) -> u32 {
    if revive == 5 && hunter_level == ORIGINAL_HUNTER_MAX_STORED_LEVEL && stage_level >= 6 {
        if stage_level == 6 {
            100
        } else {
            125
        }
    } else {
        75
    }
}

#[cfg(test)]
mod tests {
    use super::{
        original_apply_experience, original_hunter_display_level, original_need_experience,
        original_secondary_progression_base, ORIGINAL_HUNTER_MAX_DISPLAY_LEVEL,
        ORIGINAL_HUNTER_MAX_STORED_LEVEL,
    };

    #[test]
    fn get_need_exp_uses_level_plus_one_and_revive_column() {
        assert_eq!(original_need_experience(0, 0), Some(240));
        assert_eq!(original_need_experience(0, 1), Some(243));
        assert_eq!(original_need_experience(1, 0), Some(960));
        assert_eq!(original_need_experience(5, 0), Some(5_529_600));
        assert_eq!(original_need_experience(99, 0), Some(5_529_600));
    }

    #[test]
    fn exact_threshold_does_not_level_until_experience_exceeds_it() {
        assert_eq!(original_apply_experience(0, 0, 240, 0), Some((0, 240)));
        assert_eq!(original_apply_experience(0, 0, 241, 0), Some((1, 1)));
    }

    #[test]
    fn carry_can_cross_multiple_catalog_levels() {
        // 241 crosses level 0, then 243 crosses level 1, leaving one EXP.
        assert_eq!(original_apply_experience(0, 0, 484, 0), Some((2, 1)));
    }

    #[test]
    fn maximum_level_discards_the_incoming_grant() {
        assert_eq!(ORIGINAL_HUNTER_MAX_STORED_LEVEL, 99);
        assert_eq!(ORIGINAL_HUNTER_MAX_DISPLAY_LEVEL, 100);
        assert_eq!(original_hunter_display_level(99), 100);
        assert_eq!(original_apply_experience(99, 77, 999, 0), Some((99, 77)));
        assert_eq!(original_apply_experience(98, 0, u64::MAX, 0), Some((99, 0)));
    }

    #[test]
    fn secondary_progression_selector_is_not_an_alternate_level_cap() {
        assert_eq!(original_secondary_progression_base(4, 99, 6), 75);
        assert_eq!(original_secondary_progression_base(5, 98, 6), 75);
        assert_eq!(original_secondary_progression_base(5, 99, 5), 75);
        assert_eq!(original_secondary_progression_base(5, 99, 6), 100);
        assert_eq!(original_secondary_progression_base(5, 99, 7), 125);
    }
}
