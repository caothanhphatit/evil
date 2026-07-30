use super::{
    DurableHunterRosterState, HunterAgentState, MonsterActionState, MonsterMapConfig, MonsterState,
    NavigationObstacle, RegionBounds, ENTRY_CHECKPOINT_TOLERANCE_PX, HUNTER_MELEE_ATTACK_RANGE_PX,
    HUNTER_MOVE_PX_PER_TWO_TICKS, HUNTER_RANGED_ATTACK_RANGE_PX, MONSTER_MOVE_PX_PER_TICK,
    MONSTER_PATROL_IDLE_TICKS, MONSTER_PATROL_RADIUS_PX, TOWN_ROAM_ANCHORS, TOWN_ROAM_BOUNDS,
    TOWN_ROAM_IDLE_VARIANCE_TICKS, TOWN_ROAM_MIN_IDLE_TICKS,
};

pub(super) fn spawn_point(bounds: RegionBounds, index: usize) -> (i32, i32) {
    let columns = 3_i32;
    let column = i32::try_from(index).unwrap_or(0) % columns;
    let row = i32::try_from(index).unwrap_or(0) / columns;
    let horizontal_step = (bounds.max_x - bounds.min_x - 120) / 3;
    let vertical_step = (bounds.max_y - bounds.min_y - 120) / 3;
    (
        bounds.min_x + 60 + column * horizontal_step,
        bounds.min_y + 60 + row * vertical_step,
    )
}

pub(super) fn valid_hunter_target<'a>(
    agents: &'a [HunterAgentState],
    roster: &DurableHunterRosterState,
    region_id: &str,
    hunter_id: Option<u32>,
) -> Option<&'a HunterAgentState> {
    let hunter_id = hunter_id?;
    let alive = roster
        .hunters
        .iter()
        .any(|hunter| hunter.hunter_id == hunter_id && hunter.current_hp > 0);
    alive
        .then(|| {
            agents.iter().find(|agent| {
                agent.hunter_id == hunter_id && agent.region_id.as_deref() == Some(region_id)
            })
        })
        .flatten()
}

pub(super) fn nearest_hunter<'a>(
    agents: &'a [HunterAgentState],
    roster: &DurableHunterRosterState,
    region_id: &str,
    x: i32,
    y: i32,
    range: i32,
) -> Option<&'a HunterAgentState> {
    agents
        .iter()
        .filter(|agent| {
            agent.region_id.as_deref() == Some(region_id)
                && roster
                    .hunters
                    .iter()
                    .any(|hunter| hunter.hunter_id == agent.hunter_id && hunter.current_hp > 0)
        })
        .filter(|agent| squared_distance(x, y, agent.x, agent.y) <= i64::from(range).pow(2))
        .min_by_key(|agent| squared_distance(x, y, agent.x, agent.y))
}

pub(super) fn patrol(monster: &mut MonsterState, bounds: RegionBounds) {
    if monster.action_state == MonsterActionState::Idle && monster.patrol_idle_ticks > 0 {
        monster.patrol_idle_ticks = monster.patrol_idle_ticks.saturating_sub(1);
        monster.animation = "stay".to_owned();
        return;
    }

    let waypoint = patrol_waypoint(monster, bounds);
    monster.action_state = MonsterActionState::Patrolling;
    monster.animation = monster_directional_animation("walk", monster.y, waypoint.1);
    move_toward(
        &mut monster.x,
        &mut monster.y,
        waypoint.0,
        waypoint.1,
        MONSTER_MOVE_PX_PER_TICK,
        &mut monster.facing_left,
    );
    if monster.x == waypoint.0 && monster.y == waypoint.1 {
        monster.patrol_phase = monster.patrol_phase.wrapping_add(1);
        monster.patrol_idle_ticks = MONSTER_PATROL_IDLE_TICKS;
        monster.action_state = MonsterActionState::Idle;
        monster.animation = "stay".to_owned();
    }
}

pub(super) fn patrol_waypoint(monster: &MonsterState, bounds: RegionBounds) -> (i32, i32) {
    const OFFSETS: [(i32, i32); 8] = [
        (MONSTER_PATROL_RADIUS_PX, 0),
        (45, 45),
        (0, MONSTER_PATROL_RADIUS_PX),
        (-45, 45),
        (-MONSTER_PATROL_RADIUS_PX, 0),
        (-45, -45),
        (0, -MONSTER_PATROL_RADIUS_PX),
        (45, -45),
    ];
    let offset = OFFSETS[usize::from(monster.patrol_phase) % OFFSETS.len()];
    bounds.closest_point(
        monster.spawn_x.saturating_add(offset.0),
        monster.spawn_y.saturating_add(offset.1),
        24,
    )
}

pub(super) fn move_toward(
    x: &mut i32,
    y: &mut i32,
    target_x: i32,
    target_y: i32,
    step: i32,
    facing_left: &mut bool,
) {
    let dx = target_x - *x;
    let dy = target_y - *y;
    if dx != 0 {
        *facing_left = dx < 0;
    }
    let squared = u64::try_from(i64::from(dx).pow(2) + i64::from(dy).pow(2)).unwrap_or(u64::MAX);
    let distance = integer_sqrt(squared);
    if distance <= u64::try_from(step).unwrap_or(0) {
        *x = target_x;
        *y = target_y;
        return;
    }
    let distance = i64::try_from(distance).unwrap_or(i64::MAX).max(1);
    let step = i64::from(step);
    let step_x = (i64::from(dx) * step / distance).clamp(-step, step);
    let step_y = (i64::from(dy) * step / distance).clamp(-step, step);
    *x = x.saturating_add(i32::try_from(step_x).unwrap_or(0));
    *y = y.saturating_add(i32::try_from(step_y).unwrap_or(0));
}

pub(super) fn hunter_move_step(tick: u64) -> i32 {
    // Preserve the requested 1.5x increase from 5 px/tick without rounding
    // away the half pixel in the deterministic integer simulation.
    HUNTER_MOVE_PX_PER_TWO_TICKS / 2 + i32::from(tick % 2 == 0)
}

pub(super) fn hunter_attack_range(class_family: &str) -> i32 {
    match class_family {
        "H3" | "H4" => HUNTER_RANGED_ATTACK_RANGE_PX,
        _ => HUNTER_MELEE_ATTACK_RANGE_PX,
    }
}

pub(super) fn face_toward_x(facing_left: &mut bool, current_x: i32, target_x: i32) {
    if current_x != target_x {
        *facing_left = target_x < current_x;
    }
}

// The packaged monster has explicit front/back Spine clips. The native axis
// comparator is unresolved; this rebuild policy treats a target above the
// actor in scene Y-down coordinates as the back clip.
pub(super) fn monster_directional_animation(base: &str, actor_y: i32, target_y: i32) -> String {
    if target_y < actor_y {
        format!("{base}_b")
    } else {
        base.to_owned()
    }
}

pub(super) fn move_toward_avoiding(
    x: &mut i32,
    y: &mut i32,
    target_x: i32,
    target_y: i32,
    step: i32,
    facing_left: &mut bool,
    obstacles: &[NavigationObstacle],
) {
    const CLEARANCE: i32 = 14;
    let (direct_x, direct_y) = next_step(*x, *y, target_x, target_y, step);
    let blocking = obstacles
        .iter()
        .find(|obstacle| obstacle.expanded(CLEARANCE).contains(direct_x, direct_y));
    let Some(obstacle) = blocking else {
        move_toward(x, y, target_x, target_y, step, facing_left);
        return;
    };
    let expanded = obstacle.expanded(CLEARANCE);
    let top = expanded.min_y.saturating_sub(1);
    let bottom = expanded.max_y.saturating_add(1);
    let left = expanded.min_x.saturating_sub(1);
    let right = expanded.max_x.saturating_add(1);
    let (waypoint_x, waypoint_y) = if *x == left && *y == top && target_y > expanded.max_y {
        (left, bottom)
    } else if *x == right && *y == top && target_y > expanded.max_y {
        (right, bottom)
    } else if *x == left && *y == bottom && target_y < expanded.min_y {
        (left, top)
    } else if *x == right && *y == bottom && target_y < expanded.min_y {
        (right, top)
    } else if *y <= expanded.min_y {
        (
            if target_x >= expanded.max_x {
                right
            } else {
                left
            },
            top,
        )
    } else if *y >= expanded.max_y {
        (
            if target_x >= expanded.max_x {
                right
            } else {
                left
            },
            bottom,
        )
    } else if *x <= expanded.min_x || *x >= expanded.max_x {
        let top_cost = i64::from((*y - top).abs()) + i64::from((target_y - top).abs());
        let bottom_cost = i64::from((*y - bottom).abs()) + i64::from((target_y - bottom).abs());
        (*x, if top_cost <= bottom_cost { top } else { bottom })
    } else {
        let candidates = [(left, *y), (right, *y), (*x, top), (*x, bottom)];
        candidates
            .into_iter()
            .min_by_key(|(candidate_x, candidate_y)| {
                squared_distance(*x, *y, *candidate_x, *candidate_y)
            })
            .unwrap_or((target_x, target_y))
    };
    move_toward(x, y, waypoint_x, waypoint_y, step, facing_left);
}

pub(super) fn next_step(x: i32, y: i32, target_x: i32, target_y: i32, step: i32) -> (i32, i32) {
    let mut next_x = x;
    let mut next_y = y;
    let mut ignored_facing = false;
    move_toward(
        &mut next_x,
        &mut next_y,
        target_x,
        target_y,
        step,
        &mut ignored_facing,
    );
    (next_x, next_y)
}

pub(super) fn nearest_clear_town_anchor(
    x: i32,
    y: i32,
    obstacles: &[NavigationObstacle],
) -> Option<(i32, i32)> {
    TOWN_ROAM_ANCHORS
        .into_iter()
        .filter(|(anchor_x, anchor_y)| {
            obstacles
                .iter()
                .all(|obstacle| !obstacle.expanded(14).contains(*anchor_x, *anchor_y))
        })
        .min_by_key(|(anchor_x, anchor_y)| squared_distance(x, y, *anchor_x, *anchor_y))
}

pub(super) fn initial_town_roam_idle_ticks(hunter_id: u32) -> u16 {
    u16::try_from(u64::from(hunter_id).wrapping_mul(7) % 18).unwrap_or(0)
}

pub(super) fn town_roam_anchor_index(hunter_id: u32, sequence: u32) -> usize {
    const STRIDES: [usize; 4] = [1, 3, 5, 7];
    let hunter_seed = u64::from(hunter_id).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let start =
        usize::try_from(hunter_seed ^ (hunter_seed >> 32)).unwrap_or(0) % TOWN_ROAM_ANCHORS.len();
    let stride = STRIDES[usize::try_from(hunter_id).unwrap_or(0) % STRIDES.len()];
    start.wrapping_add(usize::try_from(sequence).unwrap_or(0).wrapping_mul(stride))
        % TOWN_ROAM_ANCHORS.len()
}

pub(super) fn town_roam_idle_ticks(hunter_id: u32, sequence: u32) -> u16 {
    let mixed = u64::from(hunter_id)
        .wrapping_mul(0x517c_c1b7_2722_0a95)
        .wrapping_add(u64::from(sequence).wrapping_mul(0x6eed_0e9d_a4d9_4a4f));
    TOWN_ROAM_MIN_IDLE_TICKS
        + u16::try_from(mixed % u64::from(TOWN_ROAM_IDLE_VARIANCE_TICKS)).unwrap_or(0)
}

pub(super) fn entry_checkpoint_stage(config: &MonsterMapConfig, x: i32, y: i32) -> Option<u8> {
    if config.bounds.contains(x, y) {
        return Some(u8::try_from(config.entry_waypoints.len()).unwrap_or(u8::MAX));
    }
    for (index, (waypoint_x, waypoint_y)) in config.entry_waypoints.iter().enumerate().rev() {
        if squared_distance(x, y, *waypoint_x, *waypoint_y)
            <= i64::from(ENTRY_CHECKPOINT_TOLERANCE_PX).pow(2)
        {
            return u8::try_from(index + 1).ok();
        }
    }
    TOWN_ROAM_BOUNDS.contains(x, y).then_some(0)
}

impl NavigationObstacle {
    pub(super) fn expanded(self, amount: i32) -> Self {
        Self {
            min_x: self.min_x.saturating_sub(amount),
            max_x: self.max_x.saturating_add(amount),
            min_y: self.min_y.saturating_sub(amount),
            max_y: self.max_y.saturating_add(amount),
        }
    }

    pub(super) fn contains(self, x: i32, y: i32) -> bool {
        (self.min_x..=self.max_x).contains(&x) && (self.min_y..=self.max_y).contains(&y)
    }
}

pub(super) fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut left = 1_u64;
    let mut right = value.min(u64::from(u32::MAX));
    while left <= right {
        let middle = left + (right - left) / 2;
        if middle <= value / middle {
            left = middle.saturating_add(1);
        } else {
            right = middle.saturating_sub(1);
        }
    }
    right
}

pub(super) fn squared_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i64 {
    i64::from(x2 - x1).pow(2) + i64::from(y2 - y1).pow(2)
}
