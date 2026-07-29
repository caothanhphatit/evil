#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BasicHunterSkillDefinition {
    pub(crate) skill_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) class_id: &'static str,
    pub(crate) class_family: &'static str,
    pub(crate) cooldown_ms: u64,
    pub(crate) confirmed_icon_path: Option<&'static str>,
}

pub(crate) fn definition(skill_id: &str) -> Option<BasicHunterSkillDefinition> {
    Some(match skill_id {
        "skill_h1_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h1_01",
            display_name: "Fury",
            class_id: "h1",
            class_family: "H1",
            cooldown_ms: 15_000,
            confirmed_icon_path: Some("sprites/skill_h1_01__1395.png"),
        },
        "skill_h1_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h1_02",
            display_name: "War Cry",
            class_id: "h1",
            class_family: "H1",
            cooldown_ms: 16_000,
            confirmed_icon_path: Some("sprites/skill_h1_02__5620.png"),
        },
        "skill_h2_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h2_01",
            display_name: "Holy Light",
            class_id: "h2",
            class_family: "H2",
            cooldown_ms: 8_000,
            confirmed_icon_path: None,
        },
        "skill_h2_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h2_02",
            display_name: "Barrier",
            class_id: "h2",
            class_family: "H2",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h3_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h3_01",
            display_name: "Multishot",
            class_id: "h3",
            class_family: "H3",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h3_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h3_02",
            display_name: "Dodge",
            class_id: "h3",
            class_family: "H3",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h4_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h4_01",
            display_name: "Thunderbolt",
            class_id: "h4",
            class_family: "H4",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h4_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h4_02",
            display_name: "Ice Armor",
            class_id: "h4",
            class_family: "H4",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h5_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h5_01",
            display_name: "Round Slash",
            class_id: "h5",
            class_family: "H5",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h5_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h5_02",
            display_name: "Concentrate",
            class_id: "h5",
            class_family: "H5",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::definition;

    #[test]
    fn catalog_contains_two_skills_for_each_basic_job() {
        for class in 1..=5 {
            for slot in 1..=2 {
                let skill_id = format!("skill_h{class}_{slot:02}");
                let skill = definition(&skill_id).expect("packaged basic skill");

                assert_eq!(skill.skill_id, skill_id);
                assert_eq!(skill.class_id, format!("h{class}"));
                assert_eq!(skill.class_family, format!("H{class}"));
                assert!(skill.cooldown_ms > 0);
            }
        }
    }

    #[test]
    fn only_confirmed_h1_icons_are_projected() {
        assert!(definition("skill_h1_01")
            .unwrap()
            .confirmed_icon_path
            .is_some());
        assert!(definition("skill_h1_02")
            .unwrap()
            .confirmed_icon_path
            .is_some());
        assert!(definition("skill_h2_01")
            .unwrap()
            .confirmed_icon_path
            .is_none());
        assert!(definition("unknown").is_none());
    }
}
