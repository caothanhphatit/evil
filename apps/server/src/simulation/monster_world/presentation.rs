use super::{
    CombatPresentation, CombatPresentationKind, HunterActionState, HunterAgentState,
    MONSTER_INCOMING_DAMAGE_FIXTURE_DIVISOR,
};

pub(super) fn village_hunter_entity_id(hunter_id: u32) -> String {
    format!("village-hunter-{hunter_id}")
}

/// Keeps the unresolved original runtime factor separate from the exact catalog value.
pub(super) fn fixture_monster_attack_input(catalog_damage: u64) -> Option<i64> {
    i64::try_from((catalog_damage / MONSTER_INCOMING_DAMAGE_FIXTURE_DIVISOR).max(1)).ok()
}

pub(super) fn push_combat_presentation(
    presentations: &mut Vec<CombatPresentation>,
    sequence: &mut u64,
    source_entity_id: String,
    target_entity_id: String,
    kind: CombatPresentationKind,
    amount: Option<u64>,
) {
    *sequence = sequence.wrapping_add(1);
    presentations.push(CombatPresentation {
        sequence: *sequence,
        source_entity_id,
        target_entity_id,
        kind,
        amount,
    });
}

pub(super) fn set_hunter_presentation(
    agent: &mut HunterAgentState,
    state: HunterActionState,
    animation: &str,
) {
    agent.action_state = state;
    agent.animation = animation.to_owned();
}
