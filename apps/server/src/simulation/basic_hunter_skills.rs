#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BasicHunterSkillDefinition<'a> {
    pub(crate) skill_id: &'a str,
    pub(crate) display_name: &'a str,
    pub(crate) class_id: &'a str,
    pub(crate) class_family: &'a str,
    pub(crate) cooldown_ms: u64,
    pub(crate) confirmed_icon_path: Option<&'a str>,
}

pub(crate) fn definition(skill_id: &str) -> Option<BasicHunterSkillDefinition<'static>> {
    let skill = super::hunter_content::basic_skill(skill_id)?;
    Some(BasicHunterSkillDefinition {
        skill_id: &skill.skill_id,
        display_name: &skill.display_name,
        class_id: &skill.class_id,
        class_family: &skill.class_family,
        cooldown_ms: skill.cooldown_ms,
        confirmed_icon_path: skill.confirmed_icon_path.as_deref(),
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
