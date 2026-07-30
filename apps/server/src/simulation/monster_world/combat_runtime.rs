use super::{
    add_experience, deterministic_combat_percent_roll, deterministic_roll, hunter_move_step,
    move_toward_avoiding, original_material_slot_grants, push_combat_presentation,
    resolve_original_neutral_hunter_attack, reward_operation_id, set_hunter_presentation,
    squared_distance, village_hunter_entity_id, CombatPresentationKind, DurableHunterLoot,
    DurableHunterRosterState, HunterActionState, HunterAttackSource, MonsterActionState,
    MonsterDrop, MonsterWorldState, OriginalHitPresentation, OriginalHunterAttackInputs,
    PendingOperation, HUNTER_LOOT_PICKUP_TICKS, HUNTER_MELEE_ATTACK_RANGE_PX,
    MONSTER_RESPAWN_TICKS,
};

impl MonsterWorldState {
    pub(super) fn resolve_hunter_attack(&mut self, target_id: &str, source: HunterAttackSource) {
        let multiplier = f32::from(self.damage_multiplier_stream.next_hundredths()) * 0.01_f32;
        let Some(monster) = self
            .fields
            .iter()
            .flat_map(|field| &field.monsters)
            .find(|monster| monster.entity_id == target_id)
        else {
            return;
        };
        let Some(target_armor) = i64::try_from(monster.armor).ok() else {
            return;
        };
        let Some(target_hp) = i64::try_from(monster.hp).ok() else {
            return;
        };
        let critical_roll = deterministic_combat_percent_roll(
            self.tick,
            source.hunter_id,
            source.attack_sequence,
            monster.source_index,
        );
        let Ok(result) = resolve_original_neutral_hunter_attack(OriginalHunterAttackInputs {
            calculated_damage: source.calculated_damage,
            calculated_critical_percent: source.calculated_critical_percent,
            critical_roll_zero_to_ninety_nine: critical_roll,
            conditional_critical_bonus_enabled: false,
            conditional_critical_bonus_percent: 0,
            target_armor,
            target_hp,
            hunter_feel: source.hunter_feel,
            hunter_now_feel: source.hunter_now_feel,
            rand_damage_multiplier: multiplier,
        }) else {
            return;
        };
        let Some(damage) = u64::try_from(result.final_damage).ok() else {
            return;
        };
        let kind = match result.presentation {
            OriginalHitPresentation::Normal => CombatPresentationKind::NormalDamage,
            OriginalHitPresentation::Critical => CombatPresentationKind::CriticalDamage,
            OriginalHitPresentation::Miss | OriginalHitPresentation::Evade => return,
        };
        self.apply_damage_to_monster(target_id, source.hunter_id, damage, kind);
    }

    pub(super) fn apply_damage_to_monster(
        &mut self,
        target_id: &str,
        hunter_id: u32,
        damage: u64,
        presentation_kind: CombatPresentationKind,
    ) {
        let Some(field) = self.fields.iter_mut().find(|field| {
            field
                .monsters
                .iter()
                .any(|monster| monster.entity_id == target_id)
        }) else {
            return;
        };
        let Some(monster) = field
            .monsters
            .iter_mut()
            .find(|monster| monster.entity_id == target_id)
        else {
            return;
        };
        monster.hp = monster.hp.saturating_sub(damage);
        monster.target_hunter_id = Some(hunter_id);
        push_combat_presentation(
            &mut self.combat_presentations,
            &mut self.presentation_sequence,
            village_hunter_entity_id(hunter_id),
            monster.entity_id.clone(),
            presentation_kind,
            Some(damage),
        );
        if monster.hp > 0 {
            return;
        }
        monster.action_state = MonsterActionState::Dead;
        monster.animation = "die".to_owned();
        monster.respawn_ticks = Some(MONSTER_RESPAWN_TICKS);
        monster.target_hunter_id = None;
        self.reward_sequence = self.reward_sequence.saturating_add(1);
        let drop_id = format!("drop-{}-{}", monster.entity_id, self.reward_sequence);
        let material_drops = monster
            .materials
            .iter()
            .enumerate()
            .filter_map(|(slot, material)| {
                let roll = deterministic_roll(
                    self.tick,
                    self.reward_sequence,
                    monster.source_index,
                    slot as u64,
                );
                original_material_slot_grants(material.raw_percent, roll)
                    .then_some((material.source_index, material.count))
            })
            .collect::<Vec<_>>();
        field.drops.push(MonsterDrop {
            drop_id: format!("{drop_id}-gold"),
            monster_entity_id: monster.entity_id.clone(),
            item_id: "gold".to_owned(),
            quantity: u32::try_from(monster.gold).unwrap_or(u32::MAX),
            x: monster.x - 8,
            y: monster.y,
            owner_hunter_id: hunter_id,
            gold: monster.gold,
            experience: monster.experience,
        });
        for (index, (item_index, quantity)) in material_drops.into_iter().enumerate() {
            field.drops.push(MonsterDrop {
                drop_id: format!("{drop_id}-material-{index}"),
                monster_entity_id: monster.entity_id.clone(),
                item_id: format!("material:{item_index}"),
                quantity,
                x: monster.x + i32::try_from(index).unwrap_or(0) * 8,
                y: monster.y,
                owner_hunter_id: hunter_id,
                gold: 0,
                experience: 0,
            });
        }
    }

    pub(super) fn try_collect_drop(
        &mut self,
        agent_index: usize,
        roster: &mut DurableHunterRosterState,
        operations: &mut Vec<PendingOperation>,
    ) -> bool {
        let hunter_id = self.hunters[agent_index].hunter_id;
        let Some(region_id) = self.hunters[agent_index].region_id.clone() else {
            return false;
        };
        let Some(field_index) = self
            .fields
            .iter()
            .position(|field| field.map_id == region_id)
        else {
            return false;
        };
        let candidate = self.fields[field_index]
            .drops
            .iter()
            .enumerate()
            .filter(|(_, drop)| drop.owner_hunter_id == hunter_id)
            .min_by_key(|(_, drop)| {
                squared_distance(
                    self.hunters[agent_index].x,
                    self.hunters[agent_index].y,
                    drop.x,
                    drop.y,
                )
            })
            .map(|(index, drop)| (index, drop.clone()));
        let Some((drop_index, drop)) = candidate else {
            self.hunters[agent_index].target_drop_id = None;
            return false;
        };
        if squared_distance(
            self.hunters[agent_index].x,
            self.hunters[agent_index].y,
            drop.x,
            drop.y,
        ) > i64::from(HUNTER_MELEE_ATTACK_RANGE_PX).pow(2)
        {
            let agent = &mut self.hunters[agent_index];
            agent.target_drop_id = Some(drop.drop_id.clone());
            set_hunter_presentation(agent, HunterActionState::CollectingLoot, "hunter_walk");
            move_toward_avoiding(
                &mut agent.x,
                &mut agent.y,
                drop.x,
                drop.y,
                hunter_move_step(self.tick),
                &mut agent.facing_left,
                &[],
            );
            return true;
        }
        if self.hunters[agent_index].target_drop_id.as_deref() != Some(&drop.drop_id) {
            let agent = &mut self.hunters[agent_index];
            agent.target_drop_id = Some(drop.drop_id.clone());
            agent.recovery_ticks = HUNTER_LOOT_PICKUP_TICKS;
            set_hunter_presentation(agent, HunterActionState::CollectingLoot, "hunter_stay");
            return true;
        }
        if self.hunters[agent_index].recovery_ticks > 0 {
            set_hunter_presentation(
                &mut self.hunters[agent_index],
                HunterActionState::CollectingLoot,
                "hunter_stay",
            );
            return true;
        }
        self.fields[field_index].drops.remove(drop_index);
        let Ok(hunter) = roster.active_mut(hunter_id) else {
            return true;
        };
        hunter.gold = hunter.gold.saturating_add(drop.gold);
        let credited_experience = add_experience(hunter, drop.experience);
        if credited_experience > 0 {
            push_combat_presentation(
                &mut self.combat_presentations,
                &mut self.presentation_sequence,
                drop.monster_entity_id.clone(),
                village_hunter_entity_id(hunter_id),
                CombatPresentationKind::Experience,
                Some(credited_experience),
            );
        }
        // Gold is carried in `drop.gold` and is credited directly to the Hunter wallet.
        // Only material drops belong in the sellable Hunter loot inventory.
        if drop.quantity > 0 && drop.item_id.starts_with("material:") {
            if let Some(existing) = hunter
                .hunt
                .loot
                .iter_mut()
                .find(|loot| loot.item_id == drop.item_id)
            {
                existing.quantity = existing.quantity.saturating_add(drop.quantity);
            } else {
                hunter.hunt.loot.push(DurableHunterLoot {
                    item_id: drop.item_id.clone(),
                    quantity: drop.quantity,
                });
            }
        }
        self.hunters[agent_index].loot_sequence =
            self.hunters[agent_index].loot_sequence.wrapping_add(1);
        self.hunters[agent_index].loot_item_id = Some(drop.item_id.clone());
        self.hunters[agent_index].loot_quantity = drop.quantity;
        let operation_id = reward_operation_id(self.tick, hunter_id, &drop.drop_id);
        let item_id = drop
            .item_id
            .strip_prefix("material:")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|item_id| *item_id > 0);
        if let Some(item_id) = item_id.filter(|_| drop.quantity > 0) {
            operations.push(PendingOperation::Reward {
                operation_id,
                gold: drop.gold,
                item_id,
                quantity: drop.quantity,
            });
        }
        self.hunters[agent_index].target_drop_id = None;
        true
    }
}
