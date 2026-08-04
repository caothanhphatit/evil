use super::{
    DurableHunterState, Postgres, RepositoryError, Transaction, Uuid, ACTIVE_BUILDING_RELEASE_ID,
};
use std::collections::BTreeMap;

pub(super) async fn save_hunter_owned_items_in(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
    hunter: &DurableHunterState,
) -> Result<(), RepositoryError> {
    let hunter_id = i64::from(hunter.hunter_id);
    for statement in [
        "DELETE FROM player_hunter_item_stack WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_gear_instance WHERE player_token = $1 AND hunter_id = $2",
    ] {
        sqlx::query(statement)
            .bind(player_token)
            .bind(hunter_id)
            .execute(&mut **transaction)
            .await?;
    }

    let mut stacks = BTreeMap::<&str, u64>::new();
    for item in &hunter.owned_items {
        if item.quantity == 0 {
            continue;
        }
        if let Some(gear_instance_id) = item.gear_instance_id {
            let (gear_kind, gear_index, rating) =
                parse_gear_product_id(&item.product_id).ok_or(RepositoryError::InvalidOperation)?;
            sqlx::query(
                r#"INSERT INTO player_hunter_gear_instance
                   (gear_instance_id, player_token, hunter_id, content_release_id, product_id,
                    gear_kind, gear_index, rating, enhancement_level, quality,
                    primary_stat, option_type, option_value, ruleset)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)"#,
            )
            .bind(gear_instance_id)
            .bind(player_token)
            .bind(hunter_id)
            .bind(ACTIVE_BUILDING_RELEASE_ID)
            .bind(&item.product_id)
            .bind(gear_kind)
            .bind(gear_index)
            .bind(rating)
            .bind(i16::from(item.enhancement_level.unwrap_or(0)))
            .bind(item.quality.map(i16::from))
            .bind(item.primary_stat.map(i64::from))
            .bind(item.option_type.map(i16::from))
            .bind(item.option_value.map(i32::from))
            .bind(&item.ruleset)
            .execute(&mut **transaction)
            .await?;
        } else {
            let total = stacks.entry(&item.product_id).or_default();
            *total = total
                .checked_add(u64::from(item.quantity))
                .ok_or(RepositoryError::InvalidOperation)?;
        }
    }
    let mut product_ids = Vec::with_capacity(stacks.len());
    let mut quantities = Vec::with_capacity(stacks.len());
    for (product_id, quantity) in stacks {
        product_ids.push(product_id.to_owned());
        quantities.push(i64::try_from(quantity).map_err(|_| RepositoryError::InvalidOperation)?);
    }
    if !product_ids.is_empty() {
        sqlx::query(
            r#"INSERT INTO player_hunter_item_stack
               (player_token, hunter_id, content_release_id, product_id, quantity)
               SELECT $1, $2, $3, rows.product_id, rows.quantity
               FROM UNNEST($4::text[], $5::bigint[]) AS rows(product_id, quantity)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(ACTIVE_BUILDING_RELEASE_ID)
        .bind(&product_ids)
        .bind(&quantities)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

fn parse_gear_product_id(product_id: &str) -> Option<(&str, i32, i16)> {
    let mut parts = product_id.split(':');
    let prefix = parts.next()?;
    let kind = parts.next()?;
    let index = parts.next()?.parse().ok()?;
    let rating_label = parts.next()?;
    let rating = parts.next()?.parse().ok()?;
    (prefix == "recipe" && rating_label == "rating").then_some((kind, index, rating))
}
