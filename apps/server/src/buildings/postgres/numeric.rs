use crate::buildings::{BuildingRepositoryError, BuildingSkinId};

pub(super) fn to_u64(value: i64) -> Result<u64, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

pub(super) fn to_u32(value: i64) -> Result<u32, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

pub(super) fn to_u16(value: i64) -> Result<u16, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

pub(super) fn optional_u64(value: Option<i64>) -> Result<Option<u64>, BuildingRepositoryError> {
    value.map(to_u64).transpose()
}

pub(super) fn optional_u16(value: Option<i64>) -> Result<Option<u16>, BuildingRepositoryError> {
    value.map(to_u16).transpose()
}

pub(super) fn optional_skin_id(
    value: Option<i64>,
) -> Result<Option<BuildingSkinId>, BuildingRepositoryError> {
    value
        .map(|value| BuildingSkinId::new(to_u64(value)?))
        .transpose()
}
