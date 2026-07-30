use super::{
    basic_hunter_skill_definition, HunterActionState, OriginalFlowSession, ServerMessage, Uuid,
};

impl OriginalFlowSession {
    /// Activates all ten packaged basic skills while leaving unresolved effect
    /// formulas unavailable instead of substituting guessed combat outcomes.
    pub(super) fn use_hunter_skill(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        skill_id: &str,
        target_entity_id: Option<&str>,
        produce_response: bool,
    ) -> Option<ServerMessage> {
        let key = format!("use_hunter_skill:{hunter_id}:{skill_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    produce_response.then(|| self.accepted("use_hunter_skill"))
                } else {
                    produce_response
                        .then(|| self.rejected("use_hunter_skill", "command id was already used"))
                };
            }
        }
        let Some(definition) = basic_hunter_skill_definition(skill_id) else {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill definition is unavailable"));
        };
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return produce_response.then(|| {
                self.rejected(
                    "use_hunter_skill",
                    "hunter is not in the active town roster",
                )
            });
        };
        if hunter.profile.class_id != definition.class_id
            || hunter.profile.visual_family != definition.class_family
        {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is unavailable for hunter job"));
        }
        let Some(skill) = hunter
            .profile
            .skills
            .iter()
            .find(|skill| skill.skill_id == skill_id)
        else {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is not learned"));
        };
        if !skill.ready || skill.cooldown_remaining_ms > 0 {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is on cooldown"));
        }
        if let Err(reason) = self.monster_world.validate_hunter_skill_effect(
            &self.hunter_roster,
            hunter_id,
            skill_id,
            target_entity_id,
        ) {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Err(reason) = self.monster_world.trigger_hunter_skill(
            hunter_id,
            target_entity_id,
            definition.class_family,
            definition.skill_id,
        ) {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Err(reason) =
            self.monster_world
                .apply_hunter_skill_effect(&self.hunter_roster, hunter_id, skill_id)
        {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        {
            if let Some(skill) = hunter
                .profile
                .skills
                .iter_mut()
                .find(|skill| skill.skill_id == skill_id)
            {
                skill.ready = false;
                skill.cooldown_remaining_ms = definition.cooldown_ms;
            }
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        produce_response.then(|| self.accepted("use_hunter_skill"))
    }

    pub(super) fn refresh_skill_cooldowns(&mut self, elapsed_ms: u64) {
        for hunter in &mut self.hunter_roster.hunters {
            for skill in &mut hunter.profile.skills {
                if skill.cooldown_remaining_ms == 0 {
                    skill.ready = true;
                    continue;
                }
                skill.cooldown_remaining_ms =
                    skill.cooldown_remaining_ms.saturating_sub(elapsed_ms);
                skill.ready = skill.cooldown_remaining_ms == 0;
            }
        }
    }

    pub(super) fn auto_cast_ready_hunter_skills(&mut self) {
        let casts = self
            .monster_world
            .hunters
            .iter()
            .filter_map(|agent| {
                if agent.action_state != HunterActionState::Attacking {
                    return None;
                }
                let target = agent.target_monster_id.clone()?;
                if agent.active_skill_id.is_some() || agent.region_id.is_none() {
                    return None;
                }
                let hunter = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .find(|hunter| hunter.hunter_id == agent.hunter_id)?;
                if hunter.hunt.pending_trade.is_some() || hunter.hunt.gear_enhancement.is_some() {
                    return None;
                }
                let skill = hunter
                    .profile
                    .skills
                    .iter()
                    .find(|skill| skill.ready && skill.cooldown_remaining_ms == 0)?;
                Some((agent.hunter_id, skill.skill_id.clone(), target))
            })
            .collect::<Vec<_>>();
        for (hunter_id, skill_id, target) in casts {
            let _ = self.use_hunter_skill(Uuid::nil(), hunter_id, &skill_id, Some(&target), false);
        }
    }
}
