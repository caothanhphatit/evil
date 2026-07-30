use std::sync::OnceLock;

use crate::buildings::HunterProgressionDefinition;

pub const EXPERIENCE_PROGRESSION_ID: &str = "evil-hunter-1.411.experience-runtime-v1";

struct ExperienceCatalog {
    max_stored_level: u32,
    display_level_offset: u32,
    rows: Vec<[u64; 6]>,
}

static EXPERIENCE_CATALOG: OnceLock<ExperienceCatalog> = OnceLock::new();

pub(crate) fn install_experience_catalog(
    definition: HunterProgressionDefinition,
) -> Result<(), &'static str> {
    if definition.progression_id != EXPERIENCE_PROGRESSION_ID {
        return Err("unexpected Hunter progression identity");
    }
    let display_level_offset = u32::try_from(definition.display_level_offset)
        .map_err(|_| "Hunter progression display offset is invalid")?;
    let catalog = ExperienceCatalog {
        max_stored_level: definition.max_stored_level,
        display_level_offset,
        rows: definition.experience_by_level,
    };
    if let Some(installed) = EXPERIENCE_CATALOG.get() {
        return (installed.max_stored_level == catalog.max_stored_level
            && installed.display_level_offset == catalog.display_level_offset
            && installed.rows == catalog.rows)
            .then_some(())
            .ok_or("Hunter progression was already installed with different data");
    }
    EXPERIENCE_CATALOG
        .set(catalog)
        .map_err(|_| "Hunter progression installation raced")
}

fn experience_catalog() -> &'static ExperienceCatalog {
    EXPERIENCE_CATALOG.get_or_init(test_experience_catalog)
}

#[cfg(not(test))]
fn test_experience_catalog() -> ExperienceCatalog {
    panic!("Hunter progression catalog must be installed from PostgreSQL before simulation starts")
}

#[cfg(test)]
fn test_experience_catalog() -> ExperienceCatalog {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SourceCatalog {
        rows: Vec<SourceRow>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SourceRow {
        index: u32,
        experience_by_difficulty: [u64; 6],
    }
    let source: SourceCatalog = serde_json::from_str(include_str!(
        "../../../../packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json"
    ))
    .expect("test progression fixture must decode");
    assert!(source
        .rows
        .iter()
        .enumerate()
        .all(|(index, row)| row.index == index as u32));
    ExperienceCatalog {
        max_stored_level: 99,
        display_level_offset: 1,
        rows: source
            .rows
            .into_iter()
            .map(|row| row.experience_by_difficulty)
            .collect(),
    }
}

pub fn original_hunter_max_stored_level() -> u32 {
    experience_catalog().max_stored_level
}

#[allow(dead_code)]
pub fn original_hunter_max_display_level() -> u32 {
    original_hunter_max_stored_level().wrapping_add(experience_catalog().display_level_offset)
}

/// Exact presentation/mission projection used by the recovered `PlusExp` path.
#[allow(dead_code)]
pub fn original_hunter_display_level(stored_level: u32) -> u32 {
    stored_level.wrapping_add(experience_catalog().display_level_offset)
}

/// Exact `GameManager.GetNeedExp(revive, currentLevel)` table lookup.
/// Selectors outside `0..=4` take the native exp6/default branch.
#[allow(dead_code)] // Kept disconnected until the original global max-level source is recovered.
pub fn original_need_experience(revive: u32, current_level: u32) -> Option<u64> {
    let row_index = current_level.checked_add(1)?;
    let row = experience_catalog()
        .rows
        .get(usize::try_from(row_index).ok()?)?;
    let column = if revive <= 4 {
        usize::try_from(revive).ok()?
    } else {
        5
    };
    Some(row[column])
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
    let maximum_level = original_hunter_max_stored_level();
    if level >= maximum_level {
        return Some((level, current_experience));
    }

    loop {
        let required = original_need_experience(revive, level)?;
        let missing = required.saturating_sub(current_experience);
        if incoming_experience > missing {
            incoming_experience -= missing;
            level = level.saturating_add(1);
            current_experience = 0;
            if level >= maximum_level {
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
    if revive == 5 && hunter_level == original_hunter_max_stored_level() && stage_level >= 6 {
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
        original_apply_experience, original_hunter_display_level,
        original_hunter_max_display_level, original_hunter_max_stored_level,
        original_need_experience, original_secondary_progression_base,
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
        assert_eq!(original_hunter_max_stored_level(), 99);
        assert_eq!(original_hunter_max_display_level(), 100);
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
