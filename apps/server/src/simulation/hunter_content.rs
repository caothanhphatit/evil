use std::sync::OnceLock;

use crate::buildings::{
    HunterBasicSkillContentDefinition, HunterClassContentDefinition, HunterRarityContentDefinition,
    HunterStaticContent,
};

static HUNTER_CONTENT: OnceLock<HunterStaticContent> = OnceLock::new();

pub(crate) fn install(content: HunterStaticContent) -> Result<(), &'static str> {
    if content.classes.len() != 5
        || content.rarities.len() != 5
        || content.personalities.len() != 33
        || content.basic_skills.len() != 10
    {
        return Err("Hunter static content is incomplete");
    }
    if let Some(installed) = HUNTER_CONTENT.get() {
        return (installed == &content)
            .then_some(())
            .ok_or("Hunter static content was already installed with different data");
    }
    HUNTER_CONTENT
        .set(content)
        .map_err(|_| "Hunter static content installation raced")
}

fn content() -> &'static HunterStaticContent {
    HUNTER_CONTENT.get_or_init(test_content)
}

pub(crate) fn classes() -> &'static [HunterClassContentDefinition] {
    &content().classes
}

pub(crate) fn rarities() -> &'static [HunterRarityContentDefinition] {
    &content().rarities
}

pub(crate) fn personalities() -> &'static [String] {
    &content().personalities
}

pub(crate) fn basic_skill(skill_id: &str) -> Option<&'static HunterBasicSkillContentDefinition> {
    content()
        .basic_skills
        .iter()
        .find(|skill| skill.skill_id == skill_id)
}

#[cfg(not(test))]
fn test_content() -> HunterStaticContent {
    panic!("Hunter static content must be installed from PostgreSQL before simulation starts")
}

#[cfg(test)]
fn test_content() -> HunterStaticContent {
    let classes = [
        ("h1", "Berserker", "H1"),
        ("h2", "Paladin", "H2"),
        ("h3", "Ranger", "H3"),
        ("h4", "Sorcerer", "H4"),
        ("h5", "DarkKnight", "H5"),
    ]
    .into_iter()
    .map(
        |(class_id, display_name, visual_family)| HunterClassContentDefinition {
            class_id: class_id.to_owned(),
            display_name: display_name.to_owned(),
            visual_family: visual_family.to_owned(),
        },
    )
    .collect();
    let rarities = [
        ("normal", "Normal"),
        ("rare", "Rare"),
        ("superior", "Superior"),
        ("heroic", "Heroic"),
        ("legendary", "Legendary"),
    ]
    .into_iter()
    .map(|(rarity_id, display_name)| HunterRarityContentDefinition {
        rarity_id: rarity_id.to_owned(),
        display_name: display_name.to_owned(),
    })
    .collect();
    let personalities = [
        "Strong",
        "Fast Runner",
        "Swift",
        "Fragile",
        "Sluggish",
        "Thickheaded",
        "Careless",
        "Stingy",
        "Charismatic",
        "Dead Weight",
        "Baggy Eyes",
        "Energetic",
        "Overweight",
        "Skinny",
        "Optimistic",
        "Pessimistic",
        "Coward",
        "Fearless",
        "Addict",
        "Scared of Hospital",
        "Heroic",
        "Rich",
        "Gambler",
        "Man of Steel",
        "Nimble",
        "Laggard",
        "Sharp",
        "Dull",
        "Ordinary",
        "YOLO",
        "Internet Troll",
        "Naughty",
        "Rude",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let skills = [
        (
            "skill_h1_01",
            "Fury",
            "h1",
            "H1",
            15_000,
            Some("sprites/skill_h1_01__1395.png"),
        ),
        (
            "skill_h1_02",
            "War Cry",
            "h1",
            "H1",
            16_000,
            Some("sprites/skill_h1_02__5620.png"),
        ),
        ("skill_h2_01", "Holy Light", "h2", "H2", 8_000, None),
        ("skill_h2_02", "Barrier", "h2", "H2", 16_000, None),
        ("skill_h3_01", "Multishot", "h3", "H3", 6_000, None),
        ("skill_h3_02", "Dodge", "h3", "H3", 16_000, None),
        ("skill_h4_01", "Thunderbolt", "h4", "H4", 6_000, None),
        ("skill_h4_02", "Ice Armor", "h4", "H4", 16_000, None),
        ("skill_h5_01", "Round Slash", "h5", "H5", 6_000, None),
        ("skill_h5_02", "Concentrate", "h5", "H5", 16_000, None),
    ]
    .into_iter()
    .map(
        |(skill_id, display_name, class_id, class_family, cooldown_ms, icon)| {
            HunterBasicSkillContentDefinition {
                skill_id: skill_id.to_owned(),
                display_name: display_name.to_owned(),
                class_id: class_id.to_owned(),
                class_family: class_family.to_owned(),
                cooldown_ms,
                confirmed_icon_path: icon.map(str::to_owned),
            }
        },
    )
    .collect();
    HunterStaticContent {
        classes,
        rarities,
        personalities,
        basic_skills: skills,
    }
}
