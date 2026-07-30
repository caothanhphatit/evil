use super::{
    deterministic_combat_percent_roll, face_toward_x, hunter_attack_range, set_hunter_presentation,
    squared_distance, CombatPresentationKind, DurableHunterRosterState, HashSet, HunterActionState,
    HunterAgentState, MonsterWorldState, HUNTER_RANGED_ATTACK_RANGE_PX, HUNTER_RESPAWN_TICKS,
    HUNTER_SKILL_PRESENTATION_TICKS,
};

impl MonsterWorldState {
    pub fn restore_hunter_runtime(
        &mut self,
        roster: &DurableHunterRosterState,
        persisted: Vec<HunterAgentState>,
    ) {
        let mut seen = HashSet::new();
        self.hunters = persisted
            .into_iter()
            .filter(|agent| seen.insert(agent.hunter_id))
            .collect();
        self.reconcile_hunters(roster, &[]);

        let live_monsters = self
            .fields
            .iter()
            .flat_map(|field| {
                field
                    .monsters
                    .iter()
                    .filter(|monster| monster.hp > 0)
                    .map(|monster| (field.map_id.clone(), monster.entity_id.clone()))
            })
            .collect::<HashSet<_>>();

        for agent in &mut self.hunters {
            let Some(hunter) = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == agent.hunter_id)
            else {
                continue;
            };
            if hunter.current_hp == 0 {
                agent.action_state = HunterActionState::Dead;
                agent.animation = "hunter_die".to_owned();
                agent.target_monster_id = None;
                agent.target_drop_id = None;
                agent.respawn_ticks.get_or_insert(HUNTER_RESPAWN_TICKS);
                continue;
            }

            agent.respawn_ticks = None;
            if agent.region_id.is_none() {
                agent.target_monster_id = None;
                agent.target_drop_id = None;
                if agent.action_state != HunterActionState::TownIdle {
                    set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                }
                continue;
            }

            // Ground drops are deliberately ephemeral. A reconnect can resume
            // the Hunter's location and combat target, but cannot resume a
            // collection action whose referenced drop no longer exists.
            if agent.action_state == HunterActionState::CollectingLoot {
                agent.target_drop_id = None;
                agent.loot_item_id = None;
                agent.loot_quantity = 0;
                agent.recovery_ticks = 0;
                set_hunter_presentation(agent, HunterActionState::AcquiringTarget, "hunter_stay");
            }

            let target_is_live = agent
                .region_id
                .as_ref()
                .zip(agent.target_monster_id.as_ref())
                .is_some_and(|(region_id, target_id)| {
                    live_monsters.contains(&(region_id.clone(), target_id.clone()))
                });
            if !target_is_live {
                agent.target_monster_id = None;
                if matches!(
                    agent.action_state,
                    HunterActionState::Chasing | HunterActionState::Attacking
                ) {
                    set_hunter_presentation(
                        agent,
                        HunterActionState::AcquiringTarget,
                        "hunter_stay",
                    );
                }
            }
            if agent.action_state == HunterActionState::TownIdle {
                set_hunter_presentation(agent, HunterActionState::EnteringRegion, "hunter_walk");
                agent.entry_stage = 0;
            } else if agent.action_state == HunterActionState::Dead {
                set_hunter_presentation(agent, HunterActionState::AcquiringTarget, "hunter_stay");
            }
        }
    }

    /// Starts a server-authoritative Hunter skill presentation. Exact target
    /// requirements, effect formulas and animation bindings remain unresolved,
    /// so activation validates an optional target without inventing outcomes.
    pub fn trigger_hunter_skill(
        &mut self,
        hunter_id: u32,
        target_entity_id: Option<&str>,
        class_family: &str,
        skill_id: &str,
    ) -> Result<(), &'static str> {
        let agent_index = self
            .hunters
            .iter()
            .position(|agent| agent.hunter_id == hunter_id)
            .ok_or("hunter is not in the world")?;
        let target_id = target_entity_id.map(str::to_owned);
        if let Some(target_id) = target_id {
            let region_id = self.hunters[agent_index]
                .region_id
                .as_deref()
                .ok_or("hunter is not assigned to a hunting region")?;
            let Some((target_x, target_y)) = self.monster_position_in_region(region_id, &target_id)
            else {
                return Err("skill target is unavailable");
            };
            let distance = squared_distance(
                self.hunters[agent_index].x,
                self.hunters[agent_index].y,
                target_x,
                target_y,
            );
            if distance > i64::from(hunter_attack_range(class_family)).pow(2) {
                return Err("skill target is out of range");
            }
            let hunter_x = self.hunters[agent_index].x;
            face_toward_x(
                &mut self.hunters[agent_index].facing_left,
                hunter_x,
                target_x,
            );
            self.hunters[agent_index].target_monster_id = Some(target_id);
        }
        // Skill-to-animation/effect/projectile bindings are unresolved. Keep
        // the exact skill identity as an event key and leave presentation on a
        // neutral recovered Hunter clip instead of inventing a mapping.
        set_hunter_presentation(
            &mut self.hunters[agent_index],
            HunterActionState::Attacking,
            "hunter_stay",
        );
        self.hunters[agent_index].active_skill_id = Some(skill_id.to_owned());
        self.hunters[agent_index].recovery_ticks = HUNTER_SKILL_PRESENTATION_TICKS;
        self.hunters[agent_index].attack_sequence =
            self.hunters[agent_index].attack_sequence.wrapping_add(1);
        Ok(())
    }

    pub fn apply_hunter_skill_effect(
        &mut self,
        roster: &DurableHunterRosterState,
        hunter_id: u32,
        skill_id: &str,
    ) -> Result<(), &'static str> {
        let agent_index = self
            .hunters
            .iter()
            .position(|agent| agent.hunter_id == hunter_id)
            .ok_or("hunter is not in the world")?;
        let hunter = roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .ok_or("hunter is not in the roster")?;
        let base_dps = hunter
            .profile
            .dps_milli
            .unwrap_or(hunter.profile.attack * 1_000)
            / 1_000;
        let target_id = self.hunters[agent_index].target_monster_id.clone();
        match skill_id {
            "skill_h1_01" => {
                let agent = &mut self.hunters[agent_index];
                agent.skill_buff_ticks = 100;
                agent.skill_attack_percent = 10;
                agent.skill_attack_speed_milli = 2_380;
            }
            "skill_h1_02" => {
                let Some(target_id) = target_id else {
                    return Err("skill target is unavailable");
                };
                if deterministic_combat_percent_roll(
                    self.tick,
                    hunter_id,
                    self.hunters[agent_index].attack_sequence,
                    1,
                ) < 18
                {
                    if let Some(monster) = self
                        .fields
                        .iter_mut()
                        .flat_map(|field| &mut field.monsters)
                        .find(|monster| monster.entity_id == target_id)
                    {
                        monster.stun_ticks = 30;
                    }
                }
            }
            "skill_h2_01" => self.apply_skill_aoe(hunter_id, base_dps, 430, 1),
            "skill_h2_02" => {
                let agent = &mut self.hunters[agent_index];
                agent.skill_buff_ticks = 70;
                agent.skill_defense_percent = 36;
            }
            "skill_h3_01" => {
                let Some(target_id) = target_id else {
                    return Err("skill target is unavailable");
                };
                let damage = base_dps.saturating_mul(143) / 100;
                for _ in 0..4 {
                    self.apply_damage_to_monster(
                        &target_id,
                        hunter_id,
                        damage,
                        CombatPresentationKind::NormalDamage,
                    );
                }
            }
            "skill_h3_02" => {
                let agent = &mut self.hunters[agent_index];
                agent.skill_buff_ticks = 70;
                agent.skill_evasion_percent = 20;
            }
            "skill_h4_01" | "skill_h5_01" => self.apply_skill_aoe(hunter_id, base_dps, 300, 1),
            "skill_h4_02" => {
                let agent = &mut self.hunters[agent_index];
                agent.skill_buff_ticks = 70;
                agent.ice_armor_active = true;
            }
            "skill_h5_02" => {
                let agent = &mut self.hunters[agent_index];
                agent.skill_buff_ticks = 70;
                agent.skill_critical_percent = 12;
            }
            _ => return Err("skill effect is unavailable"),
        }
        Ok(())
    }

    pub fn validate_hunter_skill_effect(
        &self,
        roster: &DurableHunterRosterState,
        hunter_id: u32,
        skill_id: &str,
        target_entity_id: Option<&str>,
    ) -> Result<(), &'static str> {
        let agent = self
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == hunter_id)
            .ok_or("hunter is not in the world")?;
        roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .ok_or("hunter is not in the roster")?;
        if !matches!(
            skill_id,
            "skill_h1_01"
                | "skill_h1_02"
                | "skill_h2_01"
                | "skill_h2_02"
                | "skill_h3_01"
                | "skill_h3_02"
                | "skill_h4_01"
                | "skill_h4_02"
                | "skill_h5_01"
                | "skill_h5_02"
        ) {
            return Err("skill effect is unavailable");
        }
        if matches!(skill_id, "skill_h1_02" | "skill_h3_01")
            && target_entity_id
                .or(agent.target_monster_id.as_deref())
                .is_none()
        {
            return Err("skill target is unavailable");
        }
        Ok(())
    }

    pub(super) fn apply_skill_aoe(
        &mut self,
        hunter_id: u32,
        base_dps: u64,
        percent: u64,
        hits: u32,
    ) {
        let Some(agent) = self
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == hunter_id)
        else {
            return;
        };
        let targets = self
            .fields
            .iter()
            .flat_map(|field| &field.monsters)
            .filter(|monster| {
                monster.hp > 0
                    && squared_distance(agent.x, agent.y, monster.x, monster.y)
                        <= i64::from(HUNTER_RANGED_ATTACK_RANGE_PX).pow(2)
            })
            .map(|monster| monster.entity_id.clone())
            .collect::<Vec<_>>();
        let damage = base_dps.saturating_mul(percent) / 100;
        for target in targets {
            for _ in 0..hits {
                self.apply_damage_to_monster(
                    &target,
                    hunter_id,
                    damage,
                    CombatPresentationKind::NormalDamage,
                );
            }
        }
    }
}
