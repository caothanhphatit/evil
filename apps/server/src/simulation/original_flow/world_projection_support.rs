use super::{
    BindingConfidence, EvidenceBinding, Facing, HunterAgentState, MonsterActionState, MonsterState,
    WorldEntityActionState, WorldEntityDescriptor, WorldEntityKind, WorldEntityProjection,
};

pub(super) fn binding(
    id: &'static str,
    confidence: BindingConfidence,
    resolved: bool,
) -> EvidenceBinding {
    EvidenceBinding {
        id,
        confidence,
        resolved,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visual_entity(
    entity_id: impl Into<String>,
    kind: WorldEntityKind,
    asset_bundle_id: &'static str,
    source_skeleton_name: &'static str,
    source_confidence: BindingConfidence,
    x: i32,
    y: i32,
    facing: Facing,
    action_state: WorldEntityActionState,
    animation: impl Into<String>,
) -> WorldEntityProjection {
    WorldEntityProjection {
        descriptor: WorldEntityDescriptor {
            entity_id: entity_id.into(),
            kind,
            asset_bundle_id,
            source_skeleton_name,
            role: "migration_visual_candidate",
            source_binding: binding("actor.spine_bundle", source_confidence, true),
            // Exact legacy spawn coordinates are still unavailable; these anchors are presentation-only.
            placement_binding: binding("actor.world_placement", BindingConfidence::Unknown, false),
        },
        x,
        y,
        facing,
        action_state,
        animation: animation.into(),
        class_family: None,
        target_entity_id: None,
        action_sequence: 0,
        loot_sequence: 0,
        loot_label: None,
        speech_label: None,
        trade_sequence: 0,
        trade_gold: 0,
        trade_materials: Vec::new(),
        attack_effect_key: None,
        skill_presentation_key: None,
        current_hp: None,
        maximum_hp: None,
        interaction_prompt_key: None,
        selectable: true,
    }
}

pub(super) fn monster_visual_entity(monster: &MonsterState) -> WorldEntityProjection {
    let family = match monster.asset_bundle_id.as_str() {
        "mon_goldblin" => "mon_goldblin",
        _ => "mon_a_01_1",
    };
    let animation = match monster.animation.as_str() {
        "atk" => "atk",
        "atk_b" => "atk_b",
        "die" => "die",
        "walk" => "walk",
        "walk_b" => "walk_b",
        _ => "stay",
    };
    let mut entity = visual_entity(
        monster.entity_id.clone(),
        WorldEntityKind::Monster,
        family,
        family,
        BindingConfidence::Confirmed,
        monster.x,
        monster.y,
        if monster.facing_left {
            Facing::Left
        } else {
            Facing::Right
        },
        match monster.action_state {
            MonsterActionState::Idle => WorldEntityActionState::Idle,
            MonsterActionState::Patrolling | MonsterActionState::Chasing
                if monster.animation == "walk" || monster.animation == "walk_b" =>
            {
                WorldEntityActionState::Walking
            }
            MonsterActionState::Attacking => WorldEntityActionState::Attacking,
            MonsterActionState::Dead => WorldEntityActionState::Dead,
            MonsterActionState::Patrolling | MonsterActionState::Chasing => {
                WorldEntityActionState::Idle
            }
        },
        animation,
    );
    // Monsters remain server-owned combat actors, but are not player-selectable UI entities.
    entity.selectable = false;
    entity.target_entity_id = monster.target_hunter_id.map(village_hunter_entity_id);
    entity.action_sequence = monster.attack_sequence;
    entity.current_hp = Some(monster.hp);
    entity.maximum_hp = Some(monster.max_hp);
    entity
}

pub(super) fn monster_action_name(state: MonsterActionState) -> &'static str {
    match state {
        MonsterActionState::Idle => "idle",
        MonsterActionState::Patrolling => "patrolling",
        MonsterActionState::Chasing => "chasing",
        MonsterActionState::Attacking => "attacking",
        MonsterActionState::Dead => "dead",
    }
}

pub(super) fn hunter_visual_entity(
    agent: &HunterAgentState,
    current_hp: u64,
    maximum_hp: u64,
) -> WorldEntityProjection {
    use super::HunterActionState;
    let mut entity = visual_entity(
        village_hunter_entity_id(agent.hunter_id),
        WorldEntityKind::Hunter,
        "hunter",
        "hunter",
        BindingConfidence::Confirmed,
        agent.x,
        agent.y,
        if agent.facing_left {
            Facing::Left
        } else {
            Facing::Right
        },
        match agent.action_state {
            HunterActionState::EnteringRegion
            | HunterActionState::Chasing
            | HunterActionState::CollectingLoot => WorldEntityActionState::Walking,
            HunterActionState::Attacking => WorldEntityActionState::Attacking,
            HunterActionState::Dead => WorldEntityActionState::Dead,
            HunterActionState::TownIdle if agent.animation == "hunter_walk" => {
                WorldEntityActionState::Walking
            }
            HunterActionState::TownIdle | HunterActionState::AcquiringTarget => {
                WorldEntityActionState::Idle
            }
        },
        agent.animation.clone(),
    );
    entity.target_entity_id = agent.target_monster_id.clone();
    entity.action_sequence = agent.attack_sequence;
    entity.loot_sequence = agent.loot_sequence;
    entity.skill_presentation_key = agent.active_skill_id.clone();
    entity.current_hp = Some(current_hp);
    entity.maximum_hp = Some(maximum_hp);
    entity
}

pub(super) fn village_hunter_entity_id(hunter_id: u32) -> String {
    format!("village-hunter-{hunter_id}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VillageHunterMotion {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) facing: Facing,
    pub(super) action_state: WorldEntityActionState,
    pub(super) animation: &'static str,
}

pub(super) fn village_hunter_motion(tick: u64, active_slot: usize) -> VillageHunterMotion {
    const WALK_TICKS: u64 = 42;
    const IDLE_TICKS: u64 = 12;
    const CYCLE_TICKS: u64 = (WALK_TICKS + IDLE_TICKS) * 2;

    let slot = active_slot as u64;
    let min_x = 1345 + i32::try_from(slot % 4).unwrap_or(0) * 145;
    let max_x = min_x + 72;
    // Separate lanes guarantee that active Hunters never share a world position.
    let y = 645 + i32::try_from(slot).unwrap_or(0) * 24;
    let phase = (tick + slot * 17) % CYCLE_TICKS;

    if phase < WALK_TICKS {
        VillageHunterMotion {
            x: interpolate_lane(min_x, max_x, phase, WALK_TICKS),
            y,
            facing: Facing::Right,
            action_state: WorldEntityActionState::Walking,
            animation: "hunter_walk",
        }
    } else if phase < WALK_TICKS + IDLE_TICKS {
        VillageHunterMotion {
            x: max_x,
            y,
            facing: Facing::Right,
            action_state: WorldEntityActionState::Idle,
            animation: "hunter_stay",
        }
    } else if phase < WALK_TICKS * 2 + IDLE_TICKS {
        VillageHunterMotion {
            x: interpolate_lane(max_x, min_x, phase - WALK_TICKS - IDLE_TICKS, WALK_TICKS),
            y,
            facing: Facing::Left,
            action_state: WorldEntityActionState::Walking,
            animation: "hunter_walk",
        }
    } else {
        VillageHunterMotion {
            x: min_x,
            y,
            facing: Facing::Left,
            action_state: WorldEntityActionState::Idle,
            animation: "hunter_stay",
        }
    }
}

pub(super) fn interpolate_lane(start: i32, end: i32, elapsed: u64, duration: u64) -> i32 {
    let delta = i64::from(end - start);
    let offset =
        delta * i64::try_from(elapsed).unwrap_or(0) / i64::try_from(duration.max(1)).unwrap_or(1);
    start + i32::try_from(offset).unwrap_or(0)
}
