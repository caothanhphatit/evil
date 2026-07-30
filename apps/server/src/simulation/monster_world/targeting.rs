use super::{squared_distance, MonsterWorldState};

impl MonsterWorldState {
    pub(super) fn valid_monster_target(
        &self,
        agent_index: usize,
        region_id: &str,
    ) -> Option<String> {
        let target_id = self.hunters[agent_index].target_monster_id.as_ref()?;
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .find(|monster| monster.entity_id == *target_id && monster.hp > 0)
            .map(|monster| monster.entity_id.clone())
    }

    pub(super) fn nearest_monster_id(&self, agent_index: usize, region_id: &str) -> Option<String> {
        let agent = &self.hunters[agent_index];
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .filter(|monster| monster.hp > 0)
            .min_by_key(|monster| squared_distance(agent.x, agent.y, monster.x, monster.y))
            .map(|monster| monster.entity_id.clone())
    }

    pub(super) fn nearest_engaged_monster_id(
        &self,
        agent_index: usize,
        region_id: &str,
    ) -> Option<String> {
        let agent = &self.hunters[agent_index];
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .filter(|monster| monster.hp > 0 && monster.target_hunter_id == Some(agent.hunter_id))
            .min_by_key(|monster| squared_distance(agent.x, agent.y, monster.x, monster.y))
            .map(|monster| monster.entity_id.clone())
    }

    pub(super) fn monster_position(&self, target_id: &str) -> Option<(i32, i32)> {
        self.fields
            .iter()
            .flat_map(|field| &field.monsters)
            .find(|monster| monster.entity_id == target_id && monster.hp > 0)
            .map(|monster| (monster.x, monster.y))
    }

    pub(super) fn monster_position_in_region(
        &self,
        region_id: &str,
        target_id: &str,
    ) -> Option<(i32, i32)> {
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .find(|monster| monster.entity_id == target_id && monster.hp > 0)
            .map(|monster| (monster.x, monster.y))
    }
}
