use super::{
    deterministic_combat_percent_roll, deterministic_roll, face_toward_x,
    fixture_monster_attack_input, map_config, monster_directional_animation, move_toward,
    nearest_hunter, patrol, push_combat_presentation, resolve_original_neutral_monster_attack,
    squared_distance, valid_hunter_target, village_hunter_entity_id, CombatPresentationKind,
    DurableHunterRosterState, HunterActionState, MonsterActionState, MonsterWorldState,
    OriginalHitPresentation, OriginalMonsterAttackInputs, HUNTER_RESPAWN_TICKS,
    MONSTER_ATTACK_RANGE_PX, MONSTER_ATTACK_RECOVERY_TICKS, MONSTER_DETECTION_RANGE_PX,
    MONSTER_MOVE_PX_PER_TICK, MONSTER_PATROL_IDLE_TICKS,
};

impl MonsterWorldState {
    pub(super) fn tick_monsters(&mut self, roster: &mut DurableHunterRosterState) {
        let tick = self.tick;
        let presentations = &mut self.combat_presentations;
        let presentation_sequence = &mut self.presentation_sequence;
        let damage_multiplier_stream = &mut self.damage_multiplier_stream;
        for field in &mut self.fields {
            let Some(config) = map_config(&field.map_id) else {
                continue;
            };
            for monster in &mut field.monsters {
                monster.stun_ticks = monster.stun_ticks.saturating_sub(1);
                monster.slow_ticks = monster.slow_ticks.saturating_sub(1);
                if monster.stun_ticks > 0 {
                    monster.action_state = MonsterActionState::Idle;
                    monster.animation = "stay".to_owned();
                    continue;
                }
                if let Some(respawn) = monster.respawn_ticks.as_mut() {
                    *respawn = respawn.saturating_sub(1);
                    if *respawn == 0 {
                        monster.hp = monster.max_hp;
                        monster.x = monster.spawn_x;
                        monster.y = monster.spawn_y;
                        monster.action_state = MonsterActionState::Idle;
                        monster.animation = "stay".to_owned();
                        monster.target_hunter_id = None;
                        monster.patrol_idle_ticks = MONSTER_PATROL_IDLE_TICKS;
                        monster.respawn_ticks = None;
                    }
                    continue;
                }
                monster.recovery_ticks = monster.recovery_ticks.saturating_sub(1);
                let target = valid_hunter_target(
                    &self.hunters,
                    roster,
                    &field.map_id,
                    monster.target_hunter_id,
                )
                .or_else(|| {
                    nearest_hunter(
                        &self.hunters,
                        roster,
                        &field.map_id,
                        monster.x,
                        monster.y,
                        MONSTER_DETECTION_RANGE_PX,
                    )
                });
                monster.target_hunter_id = target.map(|target| target.hunter_id);
                let Some(target) = target else {
                    patrol(monster, config.bounds);
                    continue;
                };
                let distance = squared_distance(monster.x, monster.y, target.x, target.y);
                if distance > i64::from(MONSTER_ATTACK_RANGE_PX).pow(2) {
                    monster.action_state = MonsterActionState::Chasing;
                    monster.animation = monster_directional_animation("walk", monster.y, target.y);
                    let chase_target = config.bounds.closest_point(target.x, target.y, 24);
                    move_toward(
                        &mut monster.x,
                        &mut monster.y,
                        chase_target.0,
                        chase_target.1,
                        MONSTER_MOVE_PX_PER_TICK,
                        &mut monster.facing_left,
                    );
                    continue;
                }
                face_toward_x(&mut monster.facing_left, monster.x, target.x);
                monster.action_state = MonsterActionState::Attacking;
                monster.animation = monster_directional_animation("atk", monster.y, target.y);
                if monster.recovery_ticks > 0 {
                    continue;
                }
                monster.recovery_ticks = if monster.slow_ticks > 0 {
                    // A 30% attack-speed reduction makes the interval 1 / 0.7 times longer.
                    MONSTER_ATTACK_RECOVERY_TICKS.saturating_mul(10).div_ceil(7)
                } else {
                    MONSTER_ATTACK_RECOVERY_TICKS
                };
                monster.attack_sequence = monster.attack_sequence.wrapping_add(1);
                if let Ok(hunter) = roster.active_mut(target.hunter_id) {
                    let Some(incoming_damage) = fixture_monster_attack_input(monster.damage) else {
                        continue;
                    };
                    let Some(hunter_hp) = i64::try_from(hunter.current_hp).ok() else {
                        continue;
                    };
                    let runtime_status = hunter.runtime.status.as_ref();
                    let skill_agent = self
                        .hunters
                        .iter()
                        .find(|agent| agent.hunter_id == target.hunter_id);
                    let mut hunter_armor = runtime_status
                        .map(|status| status.armor)
                        .or_else(|| i64::try_from(hunter.profile.defense).ok())
                        .unwrap_or(0);
                    hunter_armor = hunter_armor.saturating_mul(i64::from(
                        100 + skill_agent.map_or(0, |agent| agent.skill_defense_percent),
                    )) / 100;
                    let hunter_feel = runtime_status
                        .map(|status| status.feel)
                        .unwrap_or(hunter.mood.maximum as f32);
                    let hunter_now_feel = runtime_status
                        .map(|status| status.now_feel)
                        .unwrap_or(hunter.mood.current as f32);
                    let multiplier =
                        f32::from(damage_multiplier_stream.next_hundredths()) * 0.01_f32;
                    let dodge_roll = deterministic_combat_percent_roll(
                        tick,
                        target.hunter_id,
                        tick,
                        monster.source_index,
                    );
                    let pet_dodge_roll = i32::try_from(
                        (deterministic_roll(
                            tick,
                            tick,
                            monster.source_index,
                            u64::from(target.hunter_id).wrapping_add(1),
                        ) - 1)
                            % 1000,
                    )
                    .unwrap_or(0);
                    let Ok(result) =
                        resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
                            incoming_damage,
                            rand_damage_multiplier: multiplier,
                            // No live effect-54 producer is modeled yet. Zero is
                            // the exact disabled state, not a synthesized miss roll.
                            attacker_effect_54_value: 0,
                            effect_54_roll_zero_to_ninety_nine: 0,
                            hunter_armor,
                            hunter_feel,
                            hunter_now_feel,
                            hunter_shield: 0,
                            hunter_hp,
                            hunter_calc_dodge: hunter.profile.calc_dodge().saturating_add(
                                skill_agent.map_or(0, |agent| agent.skill_evasion_percent),
                            ),
                            hunter_dodge_primary_roll_zero_to_ninety_nine: dodge_roll,
                            // Riding-pet dodge is still unresolved per Hunter.
                            hunter_riding_pet_dodge: 0,
                            hunter_riding_pet_roll_zero_to_nine_ninety_nine: pet_dodge_roll,
                        })
                    else {
                        continue;
                    };
                    hunter.current_hp = u64::try_from(result.hunter_hp).unwrap_or(0);
                    if skill_agent.is_some_and(|agent| agent.ice_armor_active) {
                        monster.slow_ticks = 50;
                    }
                    let (kind, amount) = match result.presentation {
                        OriginalHitPresentation::Normal => (
                            CombatPresentationKind::IncomingDamage,
                            u64::try_from(result.final_damage).ok(),
                        ),
                        OriginalHitPresentation::Miss => (CombatPresentationKind::Miss, None),
                        OriginalHitPresentation::Evade => (CombatPresentationKind::Evade, None),
                        OriginalHitPresentation::Critical => continue,
                    };
                    push_combat_presentation(
                        presentations,
                        presentation_sequence,
                        monster.entity_id.clone(),
                        village_hunter_entity_id(hunter.hunter_id),
                        kind,
                        amount,
                    );
                    if hunter.current_hp == 0 {
                        hunter.hunt.status = "dead".to_owned();
                        hunter.profile.action_state = "dead".to_owned();
                        hunter.profile.animation_name = "hunter_die".to_owned();
                        if let Some(agent) = self
                            .hunters
                            .iter_mut()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                        {
                            agent.action_state = HunterActionState::Dead;
                            agent.animation = "hunter_die".to_owned();
                            agent.respawn_ticks = Some(HUNTER_RESPAWN_TICKS);
                            agent.target_monster_id = None;
                        }
                    }
                }
            }
        }
    }
}
