use super::{original_status_calc_level, Uuid};

pub(super) fn deterministic_roll(
    tick: u64,
    reward_sequence: u64,
    source_index: u32,
    slot: u64,
) -> u32 {
    let mut value = tick
        ^ reward_sequence.rotate_left(17)
        ^ u64::from(source_index).rotate_left(31)
        ^ slot.rotate_left(47)
        ^ 0x9e37_79b9_7f4a_7c15;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    u32::try_from((value ^ (value >> 31)) % 10_000 + 1).unwrap_or(1)
}

pub(super) fn deterministic_combat_percent_roll(
    tick: u64,
    hunter_id: u32,
    attack_sequence: u64,
    source_index: u32,
) -> i32 {
    // Unity's Random.Range(0,100) bounds are proven. Its global PRNG state is
    // not, so the authoritative rebuild supplies a deterministic uniform roll
    // while preserving the original threshold comparison exactly.
    let roll = deterministic_roll(tick, attack_sequence, source_index, u64::from(hunter_id));
    i32::try_from((roll - 1) % 100).unwrap_or(0)
}

pub(super) fn original_level_scaled_attack(base_attack: u64, stored_level: u32) -> Option<i64> {
    let base_attack = i64::try_from(base_attack).ok()?;
    let stored_level = i32::try_from(stored_level).ok()?;
    Some((base_attack as f32 * original_status_calc_level(stored_level)) as i64)
}

pub(super) fn reward_operation_id(tick: u64, hunter_id: u32, drop_id: &str) -> Uuid {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in drop_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    Uuid::from_u128((u128::from(tick) << 64) ^ (u128::from(hunter_id) << 32) ^ u128::from(hash))
}

pub(super) fn add_experience(
    hunter: &mut super::super::hunter_roster::DurableHunterState,
    experience: u64,
) -> u64 {
    // The native PlusExp cap applies to stored HunterData.level (display is +1).
    if hunter.profile.level >= super::super::original_progression::ORIGINAL_HUNTER_MAX_STORED_LEVEL
    {
        return 0;
    }
    hunter.profile.xp = hunter.profile.xp.saturating_add(experience);
    while let Some(required) = hunter
        .profile
        .xp_to_next_level
        // Native PlusExp carries only when the post-grant remainder is
        // positive; landing exactly on the threshold stays at the level.
        .filter(|required| *required > 0 && hunter.profile.xp > *required)
    {
        hunter.profile.xp -= required;
        hunter.profile.level = hunter.profile.level.saturating_add(1);
        // Exact lookup is recovered, but the fixture class-to-job column mapping is not yet bound.
        hunter.profile.xp_to_next_level = Some(required.saturating_add(50));
    }
    experience
}
