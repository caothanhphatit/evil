use tracing::debug;

use super::super::hunter_roster::{DurableHunterState, DurableHunterTradeTask};

pub(super) struct HunterTradeWorkflow;

impl HunterTradeWorkflow {
    const RETURNING_ACTION: &'static str = "returning_to_trade";

    pub(super) fn begin(hunter: &mut DurableHunterState, task: DurableHunterTradeTask) {
        debug!(
            hunter_id = hunter.hunter_id,
            command_id = %task.command_id,
            building_instance_id = task.building_instance_id,
            "hunter trade workflow started"
        );
        hunter.hunt.zone_id = None;
        hunter.hunt.pending_trade = Some(task);
        Self::resume(hunter);
    }

    pub(super) fn normalize_restored(
        hunter: &mut DurableHunterState,
        task_is_valid: impl FnOnce(&DurableHunterTradeTask) -> bool,
    ) {
        let pending_is_valid = hunter
            .hunt
            .pending_trade
            .as_ref()
            .is_some_and(task_is_valid);
        let has_pending = hunter.hunt.pending_trade.is_some();
        let has_orphaned_action = !has_pending
            && (hunter.hunt.status == Self::RETURNING_ACTION
                || hunter.profile.action_state == Self::RETURNING_ACTION);

        match (has_pending, pending_is_valid, has_orphaned_action) {
            (true, true, _) => Self::resume(hunter),
            (true, false, _) | (false, _, true) => Self::release(hunter),
            (false, _, false) => {}
        }
    }

    pub(super) fn release(hunter: &mut DurableHunterState) {
        debug!(
            hunter_id = hunter.hunter_id,
            "hunter trade workflow released"
        );
        hunter.hunt.pending_trade = None;
        hunter.hunt.status = "idle".to_owned();
        hunter.profile.action_state = "idle".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
    }

    fn resume(hunter: &mut DurableHunterState) {
        hunter.hunt.status = Self::RETURNING_ACTION.to_owned();
        hunter.profile.action_state = Self::RETURNING_ACTION.to_owned();
        hunter.profile.animation_name = "hunter_walk".to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::HunterTradeWorkflow;
    use crate::simulation::{operational_migration_roster, DurableHunterTradeTask};

    #[test]
    fn invalid_or_orphaned_trade_state_fails_closed() {
        let mut hunter = operational_migration_roster().hunters.remove(0);
        hunter.profile.action_state = HunterTradeWorkflow::RETURNING_ACTION.to_owned();
        HunterTradeWorkflow::normalize_restored(&mut hunter, |_| true);
        assert_eq!(hunter.profile.action_state, "idle");

        hunter.hunt.pending_trade = Some(DurableHunterTradeTask::default());
        HunterTradeWorkflow::normalize_restored(&mut hunter, |_| false);
        assert!(hunter.hunt.pending_trade.is_none());
        assert_eq!(hunter.hunt.status, "idle");
    }
}
