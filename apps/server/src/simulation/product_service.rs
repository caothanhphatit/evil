use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEffectKind {
    Stamina,
    Hp,
    Satiety,
    Mood,
}

impl ServiceEffectKind {
    pub fn for_building(building_id: &str) -> Option<Self> {
        match building_id {
            "build_9" => Some(Self::Stamina),
            "build_12" => Some(Self::Hp),
            "build_13" => Some(Self::Satiety),
            "build_19" => Some(Self::Mood),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stamina => "stamina",
            Self::Hp => "hp",
            Self::Satiety => "satiety",
            Self::Mood => "mood",
        }
    }

    pub fn state_binding(self) -> &'static str {
        match self {
            Self::Stamina => "hunter_stamina_state_binding",
            Self::Hp => "hunter_health_state_binding",
            Self::Satiety => "hunter_satiety_state_binding",
            Self::Mood => "hunter_mood_state_binding",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HunterServiceGauge {
    pub current: u64,
    pub maximum: u64,
}

impl HunterServiceGauge {
    pub fn is_resolved(self) -> bool {
        self.maximum > 0 && self.current <= self.maximum
    }

    pub fn needs_service(self) -> bool {
        self.is_resolved() && self.current < self.maximum
    }

    /// Autonomous service is reserved for a critically depleted gauge. Manual
    /// service commands may still restore any non-full gauge.
    pub fn needs_autonomous_service(self) -> bool {
        self.is_resolved()
            && self.maximum > 0
            && u128::from(self.current) * 100 < u128::from(self.maximum) * 10
    }

    pub fn restore(&mut self, amount: u64) {
        self.current = self.current.saturating_add(amount).min(self.maximum);
    }
}

pub fn capacity_for_level(building_id: &str, level: u16) -> Option<u16> {
    let values: &[u16] = match building_id {
        // Recovered AdminBuildData entryCounts rows for all ProductCreatePop services.
        "build_9" | "build_12" | "build_13" | "build_19" => &[3, 4, 4, 5, 5, 6, 6],
        _ => return None,
    };
    values.get(level.saturating_sub(1) as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_service_capacity_is_shared_by_the_four_service_buildings() {
        for building_id in ["build_9", "build_12", "build_13", "build_19"] {
            assert_eq!(capacity_for_level(building_id, 1), Some(3));
            assert_eq!(capacity_for_level(building_id, 7), Some(6));
            assert_eq!(capacity_for_level(building_id, 8), None);
        }
    }

    #[test]
    fn restore_caps_at_the_authoritative_maximum() {
        let mut gauge = HunterServiceGauge {
            current: 80,
            maximum: 100,
        };
        gauge.restore(50);
        assert_eq!(gauge.current, 100);
    }
}
