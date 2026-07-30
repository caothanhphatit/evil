use std::{fmt, str::FromStr};

use uuid::Uuid;

use super::BuildingRepositoryError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaseBuildingId(String);

impl BaseBuildingId {
    pub fn parse(value: impl Into<String>) -> Result<Self, BuildingRepositoryError> {
        let value = value.into();
        let suffix = value
            .strip_prefix("build_")
            .ok_or_else(|| BuildingRepositoryError::InvalidBaseId(value.clone()))?;
        if suffix.is_empty()
            || !suffix.bytes().all(|byte| byte.is_ascii_digit())
            || (suffix.len() > 1 && suffix.starts_with('0'))
        {
            return Err(BuildingRepositoryError::InvalidBaseId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BaseBuildingId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BaseBuildingId {
    type Err = BuildingRepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildingSkinId(u64);

impl BuildingSkinId {
    pub fn new(value: u64) -> Result<Self, BuildingRepositoryError> {
        if value == 0 {
            return Err(BuildingRepositoryError::InvalidSkinId(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildingSkinKey {
    pub building_id: BaseBuildingId,
    pub skin_id: BuildingSkinId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TownBuildingInstanceId(Uuid);

impl TownBuildingInstanceId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn get(self) -> Uuid {
        self.0
    }
}
