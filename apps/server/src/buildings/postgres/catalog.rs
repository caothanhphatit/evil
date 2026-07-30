use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::Row;

use super::{
    numeric::{optional_u16, optional_u64, to_u16, to_u32, to_u64},
    PostgresBuildingRepository,
};
use crate::buildings::{
    BaseBuildingDefinition, BaseBuildingId, BuildingCapabilityDefinition, BuildingCatalog,
    BuildingGameplayCatalog, BuildingLevelDefinition, BuildingLevelPrerequisite,
    BuildingRepository, BuildingRepositoryError, BuildingSkinDefinition, BuildingSkinId,
    BuildingSkinKey, EconomyAmount, EconomyConversionOption, EconomyItemDefinition,
    EconomyProductDefinition, EconomyProductService, EconomyRandomOutput,
};

#[async_trait]
impl BuildingRepository for PostgresBuildingRepository {
    async fn load_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingCatalog, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        Self::lock_catalog(&mut transaction, true).await?;

        let actual_registry_sha256 = sqlx::query_scalar::<_, String>(
            "SELECT encode(registry_sha256, 'hex') FROM content_release \
             WHERE release_id = $1 AND lifecycle = 'active'",
        )
        .bind(expected_registry_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            BuildingRepositoryError::ActiveReleaseUnavailable(expected_registry_id.to_owned())
        })?;
        if actual_registry_sha256 != expected_registry_sha256 {
            return Err(BuildingRepositoryError::RegistryHashMismatch {
                expected: expected_registry_sha256.to_owned(),
                actual: actual_registry_sha256,
            });
        }

        let base_rows = sqlx::query(
            r#"SELECT building_id, release_id, display_name, category, source_type,
                      max_instances::bigint AS max_instances,
                      grid_width::bigint AS grid_width,
                      grid_height::bigint AS grid_height, movable, constructible,
                      base_sprite_asset_id
               FROM building_definition
               WHERE release_id = $1
               ORDER BY building_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let level_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, upgrade_duration_ms,
                      inventory_capacity, production_slots::bigint AS production_slots
               FROM building_level_definition
               WHERE release_id = $1
               ORDER BY building_id, level"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let level_cost_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, item_id, quantity
               FROM building_level_cost
               WHERE release_id = $1
               ORDER BY building_id, level, item_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let prerequisite_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, required_building_id,
                      required_level::bigint AS required_level
               FROM building_level_prerequisite
               WHERE release_id = $1
               ORDER BY building_id, level, required_building_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let skin_rows = sqlx::query(
            r#"SELECT building_id, skin_id, family, display_name,
                      required_level::bigint AS required_level,
                      visibility, asset_key, sprite_prefix, visual_resolved
               FROM building_skin_definition
               WHERE release_id = $1
               ORDER BY building_id, skin_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;

        let mut bases = Vec::with_capacity(base_rows.len());
        for row in base_rows {
            bases.push(BaseBuildingDefinition {
                id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                registry_id: row.try_get("release_id")?,
                display_name: row.try_get("display_name")?,
                category: row.try_get("category")?,
                source_type: row.try_get("source_type")?,
                max_instances: to_u32(row.try_get("max_instances")?)?,
                grid_width: to_u16(row.try_get("grid_width")?)?,
                grid_height: to_u16(row.try_get("grid_height")?)?,
                movable: row.try_get("movable")?,
                constructible: row.try_get("constructible")?,
                base_sprite_asset_id: row.try_get("base_sprite_asset_id")?,
            });
        }

        let mut levels = Vec::with_capacity(level_rows.len());
        for row in level_rows {
            levels.push(BuildingLevelDefinition {
                building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                level: to_u16(row.try_get("level")?)?,
                upgrade_duration_ms: optional_u64(row.try_get("upgrade_duration_ms")?)?,
                inventory_capacity: optional_u64(row.try_get("inventory_capacity")?)?,
                production_slots: optional_u16(row.try_get("production_slots")?)?,
                costs: Vec::new(),
                prerequisites: Vec::new(),
            });
        }
        for row in level_cost_rows {
            let building_id = BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?;
            let level = to_u16(row.try_get("level")?)?;
            levels
                .iter_mut()
                .find(|definition| {
                    definition.building_id == building_id && definition.level == level
                })
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "cost references unknown building level",
                ))?
                .costs
                .push(EconomyAmount {
                    resource_id: row.try_get("item_id")?,
                    quantity: to_u64(row.try_get("quantity")?)?,
                });
        }
        for row in prerequisite_rows {
            let building_id = BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?;
            let level = to_u16(row.try_get("level")?)?;
            levels
                .iter_mut()
                .find(|definition| {
                    definition.building_id == building_id && definition.level == level
                })
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "prerequisite references unknown building level",
                ))?
                .prerequisites
                .push(BuildingLevelPrerequisite {
                    building_id: BaseBuildingId::parse(
                        row.try_get::<String, _>("required_building_id")?,
                    )?,
                    required_level: to_u16(row.try_get("required_level")?)?,
                });
        }

        let mut skins = Vec::with_capacity(skin_rows.len());
        for row in skin_rows {
            skins.push(BuildingSkinDefinition {
                key: BuildingSkinKey {
                    building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                    skin_id: BuildingSkinId::new(to_u64(row.try_get("skin_id")?)?)?,
                },
                family: row.try_get("family")?,
                display_name: row.try_get("display_name")?,
                required_level: to_u16(row.try_get("required_level")?)?,
                visibility: row.try_get("visibility")?,
                asset_key: row.try_get("asset_key")?,
                sprite_prefix: row.try_get("sprite_prefix")?,
                visual_resolved: row.try_get("visual_resolved")?,
            });
        }
        transaction.commit().await?;

        let catalog = BuildingCatalog {
            registry_id: expected_registry_id.to_owned(),
            bases,
            levels,
            skins,
        };
        catalog.validate()?;
        if let Some(actual) = catalog.bases.first().map(|base| base.registry_id.as_str()) {
            if actual != expected_registry_id {
                return Err(BuildingRepositoryError::RegistryMismatch {
                    expected: expected_registry_id.to_owned(),
                    actual: actual.to_owned(),
                });
            }
        }
        Ok(catalog)
    }

    async fn load_gameplay_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingGameplayCatalog, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        Self::lock_catalog(&mut transaction, true).await?;
        let actual_hash = sqlx::query_scalar::<_, String>(
            "SELECT encode(registry_sha256, 'hex') FROM content_release \
             WHERE release_id = $1 AND lifecycle = 'active'",
        )
        .bind(expected_registry_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            BuildingRepositoryError::ActiveReleaseUnavailable(expected_registry_id.to_owned())
        })?;
        if actual_hash != expected_registry_sha256 {
            return Err(BuildingRepositoryError::RegistryHashMismatch {
                expected: expected_registry_sha256.to_owned(),
                actual: actual_hash,
            });
        }

        let capability_rows = sqlx::query(
            r#"SELECT capability_id, building_id, capability_kind,
                      static_data_ready, runnable
               FROM building_capability_definition
               WHERE release_id = $1
               ORDER BY building_id, capability_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let item_rows = sqlx::query(
            r#"SELECT item_id, internal_name, item_type, stack_limit,
                      town_pays_hunter_gold_per_unit
               FROM economy_item_definition
               WHERE release_id = $1
               ORDER BY item_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let localization_rows = sqlx::query(
            r#"SELECT item_id, locale, display_name
               FROM economy_item_localization
               WHERE release_id = $1
               ORDER BY item_id, locale"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let item_price_rows = sqlx::query(
            r#"SELECT item_id, price_direction, ordinal, resource_id, quantity
               FROM economy_item_price_component
               WHERE release_id = $1
               ORDER BY item_id, price_direction, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let tier_price_rows = sqlx::query(
            r#"SELECT item_id, tier, gold
               FROM economy_item_hunter_tier_price
               WHERE release_id = $1
               ORDER BY item_id, tier"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let product_rows = sqlx::query(
            r#"SELECT product_id, building_id, duration_ms, exact_mutation_ready
               FROM economy_product_definition
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let amount_rows = sqlx::query(
            r#"SELECT product_id, amount_kind, ordinal, resource_id, quantity
               FROM economy_product_amount
               WHERE release_id = $1
               ORDER BY product_id, amount_kind, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let service_rows = sqlx::query(
            r#"SELECT product_id, source_type, required_level::bigint AS required_level,
                      service_time_ms, effect_value, use_money, required_cash_count,
                      cash_completion_count, required_elemental_count,
                      elemental_completion_count
               FROM economy_product_service
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let completion_rows = sqlx::query(
            r#"SELECT product_id, ordinal, quantity
               FROM economy_product_service_completion
               WHERE release_id = $1
               ORDER BY product_id, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let conversion_rows = sqlx::query(
            r#"SELECT product_id, ordinal, input_kind, input_resource_id,
                      input_quantity, output_stock_quantity
               FROM economy_product_conversion_option
               WHERE release_id = $1
               ORDER BY product_id, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let random_rows = sqlx::query(
            r#"SELECT product_id, item_type, grade, quantity, rng_ready
               FROM economy_product_random_output
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;

        let capabilities = capability_rows
            .into_iter()
            .map(|row| {
                Ok(BuildingCapabilityDefinition {
                    capability_id: row.try_get("capability_id")?,
                    building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                    kind: row.try_get("capability_kind")?,
                    static_data_ready: row.try_get("static_data_ready")?,
                    runnable: row.try_get("runnable")?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;

        let mut items = BTreeMap::new();
        for row in item_rows {
            let item_id: String = row.try_get("item_id")?;
            items.insert(
                item_id.clone(),
                EconomyItemDefinition {
                    item_id,
                    internal_name: row.try_get("internal_name")?,
                    item_type: row.try_get("item_type")?,
                    stack_limit: optional_u64(row.try_get("stack_limit")?)?,
                    town_pays_hunter_gold_per_unit: optional_u64(
                        row.try_get("town_pays_hunter_gold_per_unit")?,
                    )?,
                    localized_names: BTreeMap::new(),
                    buy_price: Vec::new(),
                    sell_price: Vec::new(),
                    hunter_pays_town_gold_by_tier: Vec::new(),
                },
            );
        }
        for row in localization_rows {
            if let Some(item) = items.get_mut(row.try_get::<String, _>("item_id")?.as_str()) {
                item.localized_names
                    .insert(row.try_get("locale")?, row.try_get("display_name")?);
            }
        }
        for row in item_price_rows {
            let item_id: String = row.try_get("item_id")?;
            let amount = EconomyAmount {
                resource_id: row.try_get("resource_id")?,
                quantity: to_u64(row.try_get("quantity")?)?,
            };
            let item = items
                .get_mut(&item_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "item price references unknown item",
                ))?;
            match row.try_get::<String, _>("price_direction")?.as_str() {
                "buy" => item.buy_price.push(amount),
                "sell" => item.sell_price.push(amount),
                _ => {
                    return Err(BuildingRepositoryError::InvalidCatalog(
                        "invalid item price direction",
                    ))
                }
            }
        }
        for row in tier_price_rows {
            let item_id: String = row.try_get("item_id")?;
            items
                .get_mut(&item_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "tier price references unknown item",
                ))?
                .hunter_pays_town_gold_by_tier
                .push(to_u64(row.try_get("gold")?)?);
        }

        let mut products = BTreeMap::new();
        for row in product_rows {
            let product_id: String = row.try_get("product_id")?;
            products.insert(
                product_id.clone(),
                EconomyProductDefinition {
                    product_id,
                    building_id: row
                        .try_get::<Option<String>, _>("building_id")?
                        .map(BaseBuildingId::parse)
                        .transpose()?,
                    duration_ms: optional_u64(row.try_get("duration_ms")?)?,
                    exact_mutation_ready: row.try_get("exact_mutation_ready")?,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    sale_price: Vec::new(),
                    service: None,
                    conversion_options: Vec::new(),
                    random_output: None,
                },
            );
        }
        for row in amount_rows {
            let product_id: String = row.try_get("product_id")?;
            let product =
                products
                    .get_mut(&product_id)
                    .ok_or(BuildingRepositoryError::InvalidCatalog(
                        "amount references unknown product",
                    ))?;
            let amount = EconomyAmount {
                resource_id: row.try_get("resource_id")?,
                quantity: to_u64(row.try_get("quantity")?)?,
            };
            match row.try_get::<String, _>("amount_kind")?.as_str() {
                "input" => product.inputs.push(amount),
                "output" => product.outputs.push(amount),
                "sale_price" => product.sale_price.push(amount),
                _ => {
                    return Err(BuildingRepositoryError::InvalidCatalog(
                        "invalid product amount kind",
                    ))
                }
            }
        }
        for row in service_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "service references unknown product",
                ))?
                .service = Some(EconomyProductService {
                source_type: to_u64(row.try_get("source_type")?)?,
                required_level: to_u16(row.try_get("required_level")?)?,
                service_time_ms: to_u64(row.try_get("service_time_ms")?)?,
                effect_value: to_u64(row.try_get("effect_value")?)?,
                use_money: to_u64(row.try_get("use_money")?)?,
                completion_counts: Vec::new(),
                required_cash_count: to_u64(row.try_get("required_cash_count")?)?,
                cash_completion_count: to_u64(row.try_get("cash_completion_count")?)?,
                required_elemental_count: to_u64(row.try_get("required_elemental_count")?)?,
                elemental_completion_count: to_u64(row.try_get("elemental_completion_count")?)?,
            });
        }
        for row in completion_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .and_then(|product| product.service.as_mut())
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "completion references unknown service",
                ))?
                .completion_counts
                .push(to_u64(row.try_get("quantity")?)?);
        }
        for row in conversion_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "conversion references unknown product",
                ))?
                .conversion_options
                .push(EconomyConversionOption {
                    input_kind: row.try_get("input_kind")?,
                    input_resource_id: row.try_get("input_resource_id")?,
                    input_quantity: to_u64(row.try_get("input_quantity")?)?,
                    output_stock_quantity: to_u64(row.try_get("output_stock_quantity")?)?,
                });
        }
        for row in random_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "random output references unknown product",
                ))?
                .random_output = Some(EconomyRandomOutput {
                item_type: row.try_get("item_type")?,
                grade: to_u64(row.try_get("grade")?)?,
                quantity: to_u64(row.try_get("quantity")?)?,
                rng_ready: row.try_get("rng_ready")?,
            });
        }
        transaction.commit().await?;
        let gameplay = BuildingGameplayCatalog {
            registry_id: expected_registry_id.to_owned(),
            capabilities,
            items,
            products,
        };
        let catalog = self
            .load_catalog(expected_registry_id, expected_registry_sha256)
            .await?;
        gameplay.validate(&catalog)?;
        Ok(gameplay)
    }
}
