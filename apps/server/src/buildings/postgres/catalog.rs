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
    BuildingSkinKey, ConsumableProductDefinition, EconomyAmount, EconomyConversionOption,
    EconomyItemDefinition, EconomyProductDefinition, EconomyProductService, EconomyRandomOutput,
    GearProductDefinition, HunterBasicSkillContentDefinition, HunterClassContentDefinition,
    HunterProgressionDefinition, HunterRarityContentDefinition, HunterStaticContent,
    MonsterDefinition, MonsterMaterialDefinition, OrdinaryMonsterPoolDefinition,
    WorldMapDefinition,
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
        let material_rows = sqlx::query(
            r#"SELECT item_id, difficulty_rating
               FROM material_definition
               WHERE release_id = $1
               ORDER BY source_index"#,
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
        let gear_rows = sqlx::query(
            r#"SELECT binding.product_id, binding.gear_kind,
                      binding.gear_index::bigint AS gear_index,
                      binding.rating::bigint AS rating,
                      gear.difficulty_group::bigint AS difficulty_group
               FROM economy_product_gear_binding AS binding
               JOIN gear_definition AS gear
                 ON gear.release_id = binding.release_id
                AND gear.gear_kind = binding.gear_kind
                AND gear.gear_index = binding.gear_index
               WHERE binding.release_id = $1
               ORDER BY binding.product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let consumable_rows = sqlx::query(
            r#"SELECT binding.product_id,
                      binding.consumable_index::bigint AS consumable_index,
                      binding.level::bigint AS level, level_definition.keep_time_ms,
                      level_definition.keep_value, definition.cooldown_ms,
                      level_definition.price
               FROM economy_product_consumable_binding AS binding
               JOIN consumable_level_definition AS level_definition
                 ON level_definition.release_id = binding.release_id
                AND level_definition.consumable_index = binding.consumable_index
                AND level_definition.level = binding.level
               JOIN consumable_definition AS definition
                 ON definition.release_id = binding.release_id
                AND definition.consumable_index = binding.consumable_index
               WHERE binding.release_id = $1
               ORDER BY binding.product_id"#,
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
                    difficulty_rating: None,
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
        for row in material_rows {
            let item_id: String = row.try_get("item_id")?;
            items
                .get_mut(&item_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "material definition references unknown item",
                ))?
                .difficulty_rating = Some(
                u8::try_from(row.try_get::<i32, _>("difficulty_rating")?).map_err(|_| {
                    BuildingRepositoryError::InvalidCatalog("material rating is invalid")
                })?,
            );
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
        let mut gear_products = BTreeMap::new();
        for row in gear_rows {
            let product_id: String = row.try_get("product_id")?;
            gear_products.insert(
                product_id.clone(),
                GearProductDefinition {
                    product_id,
                    gear_kind: row.try_get("gear_kind")?,
                    gear_index: to_u32(row.try_get("gear_index")?)?,
                    rating: to_u16(row.try_get("rating")?)?,
                    difficulty_group: to_u16(row.try_get("difficulty_group")?)?,
                },
            );
        }
        let mut consumable_products = BTreeMap::new();
        for row in consumable_rows {
            let product_id: String = row.try_get("product_id")?;
            consumable_products.insert(
                product_id.clone(),
                ConsumableProductDefinition {
                    product_id,
                    consumable_index: to_u32(row.try_get("consumable_index")?)?,
                    level: to_u16(row.try_get("level")?)?,
                    keep_time_ms: to_u64(row.try_get("keep_time_ms")?)?,
                    keep_value: to_u64(row.try_get("keep_value")?)?,
                    cooldown_ms: to_u64(row.try_get("cooldown_ms")?)?,
                    price: to_u64(row.try_get("price")?)?,
                },
            );
        }
        transaction.commit().await?;
        let gameplay = BuildingGameplayCatalog {
            registry_id: expected_registry_id.to_owned(),
            capabilities,
            items,
            products,
            gear_products,
            consumable_products,
        };
        let catalog = self
            .load_catalog(expected_registry_id, expected_registry_sha256)
            .await?;
        gameplay.validate(&catalog)?;
        Ok(gameplay)
    }
}

impl PostgresBuildingRepository {
    pub async fn load_hunter_static_content(
        &self,
        profile_release_id: &str,
        info_release_id: &str,
    ) -> Result<HunterStaticContent, BuildingRepositoryError> {
        let class_rows = sqlx::query(
            r#"SELECT class_id, display_name, visual_family
               FROM hunter_class_definition
               WHERE release_id = $1
               ORDER BY source_job_index"#,
        )
        .bind(profile_release_id)
        .fetch_all(&self.pool)
        .await?;
        let rarity_rows = sqlx::query(
            r#"SELECT rarity_id, display_name
               FROM hunter_rarity_definition
               WHERE release_id = $1
               ORDER BY rank"#,
        )
        .bind(profile_release_id)
        .fetch_all(&self.pool)
        .await?;
        let personality_rows = sqlx::query(
            r#"SELECT display_name
               FROM hunter_characteristic_definition
               WHERE release_id = $1
               ORDER BY source_index"#,
        )
        .bind(info_release_id)
        .fetch_all(&self.pool)
        .await?;
        let skill_rows = sqlx::query(
            r#"SELECT skill.skill_id, skill.display_name, skill.class_id,
                      class.visual_family,
                      ((skill.source_parameters ->> 'coolTime')::double precision * 1000)::bigint
                          AS cooldown_ms,
                      skill.icon_path
               FROM hunter_skill_definition AS skill
               JOIN hunter_class_definition AS class
                 ON class.release_id = skill.release_id
                AND class.class_id = skill.class_id
               WHERE skill.release_id = $1
                 AND skill.skill_id ~ '^skill_h[1-5]_0[12]$'
               ORDER BY skill.skill_id"#,
        )
        .bind(profile_release_id)
        .fetch_all(&self.pool)
        .await?;

        let classes = class_rows
            .into_iter()
            .map(|row| {
                Ok(HunterClassContentDefinition {
                    class_id: row.try_get("class_id")?,
                    display_name: row.try_get("display_name")?,
                    visual_family: row.try_get("visual_family")?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let rarities = rarity_rows
            .into_iter()
            .map(|row| {
                Ok(HunterRarityContentDefinition {
                    rarity_id: row.try_get("rarity_id")?,
                    display_name: row.try_get("display_name")?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let personalities = personality_rows
            .into_iter()
            .map(|row| row.try_get("display_name"))
            .collect::<Result<Vec<String>, sqlx::Error>>()?;
        let basic_skills = skill_rows
            .into_iter()
            .map(|row| {
                Ok(HunterBasicSkillContentDefinition {
                    skill_id: row.try_get("skill_id")?,
                    display_name: row.try_get("display_name")?,
                    class_id: row.try_get("class_id")?,
                    class_family: row.try_get("visual_family")?,
                    cooldown_ms: to_u64(row.try_get("cooldown_ms")?)?,
                    confirmed_icon_path: row.try_get("icon_path")?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        if classes.len() != 5
            || rarities.len() != 5
            || personalities.len() != 33
            || basic_skills.len() != 10
            || basic_skills.iter().any(|skill| skill.cooldown_ms == 0)
        {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "Hunter static content is incomplete",
            ));
        }
        Ok(HunterStaticContent {
            classes,
            rarities,
            personalities,
            basic_skills,
        })
    }

    pub async fn load_ordinary_monster_pools(
        &self,
        expected_release_id: &str,
    ) -> Result<Vec<OrdinaryMonsterPoolDefinition>, BuildingRepositoryError> {
        let monster_rows = sqlx::query(
            r#"SELECT source_index::bigint AS source_index, hp, damage, armor,
                      experience, gold, asset_bundle_id
               FROM monster_definition
               WHERE release_id = $1
               ORDER BY source_index"#,
        )
        .bind(expected_release_id)
        .fetch_all(&self.pool)
        .await?;
        let drop_rows = sqlx::query(
            r#"SELECT monster_source_index::bigint AS monster_source_index,
                      slot, material_source_index::bigint AS material_source_index,
                      quantity::bigint AS quantity, raw_percent::bigint AS raw_percent
               FROM monster_material_drop_definition
               WHERE release_id = $1
               ORDER BY monster_source_index, slot"#,
        )
        .bind(expected_release_id)
        .fetch_all(&self.pool)
        .await?;
        let pool_rows = sqlx::query(
            r#"SELECT map_id, global_difficulty, pool_ordinal,
                      monster_source_index::bigint AS monster_source_index
               FROM ordinary_monster_pool_definition
               WHERE release_id = $1
               ORDER BY map_id, global_difficulty, pool_ordinal"#,
        )
        .bind(expected_release_id)
        .fetch_all(&self.pool)
        .await?;

        let mut monsters = BTreeMap::new();
        for row in monster_rows {
            let source_index = to_u32(row.try_get("source_index")?)?;
            monsters.insert(
                source_index,
                MonsterDefinition {
                    source_index,
                    hp: to_u64(row.try_get("hp")?)?,
                    damage: to_u64(row.try_get("damage")?)?,
                    armor: to_u64(row.try_get("armor")?)?,
                    experience: to_u64(row.try_get("experience")?)?,
                    gold: to_u64(row.try_get("gold")?)?,
                    asset_bundle_id: row.try_get("asset_bundle_id")?,
                    materials: Vec::new(),
                },
            );
        }
        for row in drop_rows {
            let monster_source_index = to_u32(row.try_get("monster_source_index")?)?;
            monsters
                .get_mut(&monster_source_index)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "monster drop references unknown monster",
                ))?
                .materials
                .push(MonsterMaterialDefinition {
                    source_index: to_u32(row.try_get("material_source_index")?)?,
                    count: to_u32(row.try_get("quantity")?)?,
                    raw_percent: to_u32(row.try_get("raw_percent")?)?,
                });
        }

        let mut pools: Vec<OrdinaryMonsterPoolDefinition> = Vec::new();
        for row in pool_rows {
            let map_id: String = row.try_get("map_id")?;
            let difficulty = u8::try_from(row.try_get::<i32, _>("global_difficulty")?)
                .map_err(|_| BuildingRepositoryError::InvalidCatalog("invalid world difficulty"))?;
            if pools
                .last()
                .is_none_or(|pool| pool.map_id != map_id || pool.global_difficulty != difficulty)
            {
                pools.push(OrdinaryMonsterPoolDefinition {
                    map_id: map_id.clone(),
                    global_difficulty: difficulty,
                    monsters: Vec::new(),
                });
            }
            let source_index = to_u32(row.try_get("monster_source_index")?)?;
            pools.last_mut().expect("pool was inserted").monsters.push(
                monsters.get(&source_index).cloned().ok_or(
                    BuildingRepositoryError::InvalidCatalog("pool references unknown monster"),
                )?,
            );
        }
        if pools.is_empty() || pools.iter().any(|pool| pool.monsters.is_empty()) {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "ordinary monster catalog is incomplete",
            ));
        }
        Ok(pools)
    }

    pub async fn load_hunter_progression(
        &self,
        expected_release_id: &str,
        progression_id: &str,
    ) -> Result<HunterProgressionDefinition, BuildingRepositoryError> {
        let definition = sqlx::query(
            r#"SELECT max_stored_level::bigint AS max_stored_level, display_level_offset
               FROM hunter_progression_definition
               WHERE release_id = $1 AND progression_id = $2"#,
        )
        .bind(expected_release_id)
        .bind(progression_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(BuildingRepositoryError::InvalidCatalog(
            "hunter progression is unavailable",
        ))?;
        let rows = sqlx::query(
            r#"SELECT level_index, revive_tier, required_experience
               FROM hunter_progression_experience
               WHERE release_id = $1 AND progression_id = $2
               ORDER BY level_index, revive_tier"#,
        )
        .bind(expected_release_id)
        .bind(progression_id)
        .fetch_all(&self.pool)
        .await?;
        let max_stored_level = to_u32(definition.try_get("max_stored_level")?)?;
        let row_count = usize::try_from(max_stored_level)
            .map_err(|_| BuildingRepositoryError::InvalidCatalog("progression cap is invalid"))?
            + 1;
        let mut experience_by_level = vec![[0_u64; 6]; row_count];
        let mut populated = vec![[false; 6]; row_count];
        for row in rows {
            let level = usize::try_from(row.try_get::<i32, _>("level_index")?).map_err(|_| {
                BuildingRepositoryError::InvalidCatalog("progression level is invalid")
            })?;
            let revive = usize::try_from(row.try_get::<i32, _>("revive_tier")?).map_err(|_| {
                BuildingRepositoryError::InvalidCatalog("progression revive tier is invalid")
            })?;
            let slot = experience_by_level
                .get_mut(level)
                .and_then(|levels| levels.get_mut(revive))
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "progression row is outside declared bounds",
                ))?;
            *slot = to_u64(row.try_get("required_experience")?)?;
            populated[level][revive] = true;
        }
        if populated.iter().flatten().any(|present| !present) {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "hunter progression catalog is incomplete",
            ));
        }
        let display_level_offset: i32 = definition.try_get("display_level_offset")?;
        if display_level_offset < 0 {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "progression display offset is negative",
            ));
        }
        Ok(HunterProgressionDefinition {
            progression_id: progression_id.to_owned(),
            max_stored_level,
            display_level_offset,
            experience_by_level,
        })
    }

    pub async fn load_world_maps(
        &self,
        expected_release_id: &str,
    ) -> Result<Vec<WorldMapDefinition>, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let map_rows = sqlx::query(
            r#"SELECT map_id, area::bigint AS area, monster_tier::bigint AS monster_tier,
                      map_asset_id, min_x, max_x, min_y, max_y
               FROM world_map_definition
               WHERE release_id = $1
               ORDER BY area, map_id"#,
        )
        .bind(expected_release_id)
        .fetch_all(&mut *transaction)
        .await?;
        let density_rows = sqlx::query(
            r#"SELECT map_id, density_level, spawn_count
               FROM world_map_density_definition
               WHERE release_id = $1
               ORDER BY map_id, density_level"#,
        )
        .bind(expected_release_id)
        .fetch_all(&mut *transaction)
        .await?;
        let waypoint_rows = sqlx::query(
            r#"SELECT map_id, ordinal, x, y
               FROM world_map_entry_waypoint
               WHERE release_id = $1
               ORDER BY map_id, ordinal"#,
        )
        .bind(expected_release_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;

        let mut maps = map_rows
            .into_iter()
            .map(|row| {
                Ok(WorldMapDefinition {
                    map_id: row.try_get("map_id")?,
                    area: u8::try_from(row.try_get::<i64, _>("area")?).map_err(|_| {
                        BuildingRepositoryError::InvalidCatalog("world map area is outside u8")
                    })?,
                    monster_tier: u8::try_from(row.try_get::<i64, _>("monster_tier")?).map_err(
                        |_| BuildingRepositoryError::InvalidCatalog("world map tier is outside u8"),
                    )?,
                    map_asset_id: row.try_get("map_asset_id")?,
                    density_counts: [0; 3],
                    bounds: (
                        row.try_get("min_x")?,
                        row.try_get("max_x")?,
                        row.try_get("min_y")?,
                        row.try_get("max_y")?,
                    ),
                    entry_waypoints: [(0, 0); 3],
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        for row in density_rows {
            let map_id: String = row.try_get("map_id")?;
            let map = maps.iter_mut().find(|map| map.map_id == map_id).ok_or(
                BuildingRepositoryError::InvalidCatalog("density references unknown map"),
            )?;
            let level = usize::try_from(row.try_get::<i32, _>("density_level")? - 1)
                .map_err(|_| BuildingRepositoryError::InvalidCatalog("invalid density level"))?;
            map.density_counts[level] = u32::try_from(row.try_get::<i32, _>("spawn_count")?)
                .map_err(|_| BuildingRepositoryError::InvalidCatalog("spawn count outside u32"))?;
        }
        for row in waypoint_rows {
            let map_id: String = row.try_get("map_id")?;
            let map = maps.iter_mut().find(|map| map.map_id == map_id).ok_or(
                BuildingRepositoryError::InvalidCatalog("waypoint references unknown map"),
            )?;
            let ordinal = usize::try_from(row.try_get::<i32, _>("ordinal")?)
                .map_err(|_| BuildingRepositoryError::InvalidCatalog("invalid waypoint ordinal"))?;
            map.entry_waypoints[ordinal] = (row.try_get("x")?, row.try_get("y")?);
        }
        if maps.is_empty()
            || maps
                .iter()
                .any(|map| map.density_counts.contains(&0) || map.entry_waypoints.contains(&(0, 0)))
        {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "world map catalog is incomplete",
            ));
        }
        Ok(maps)
    }
}
