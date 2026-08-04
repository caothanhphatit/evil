use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    simulation::{DURABLE_PLAYER_SCHEMA_VERSION, PROTOCOL_VERSION},
    AppState,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OverviewResponse {
    service: &'static str,
    status: &'static str,
    protocol_version: u16,
    durable_schema_version: u16,
    tick_rate: u32,
    players: i64,
    hunters: i64,
    items: i64,
    releases: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    #[serde(default)]
    search: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageResponse {
    data: Vec<serde_json::Value>,
    page: i64,
    page_size: i64,
    total: i64,
    total_pages: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    25
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/items", get(items))
        .route("/buildings", get(buildings))
        .route("/monsters", get(monsters))
        .route("/gear", get(gear))
        .route("/rebuild-weapons", get(rebuild_weapons))
        .route("/affixes", get(affixes))
        .route("/affix-tiers", get(affix_tiers))
        .route("/affix-pools", get(affix_pools))
        .route("/wiki/weapon-modifiers", get(weapon_modifier_wiki))
        .route("/virtues", get(virtues))
        .route("/collection-sets", get(collection_sets))
        .route("/consumables", get(consumables))
        .route("/hunters", get(hunters))
        .route("/players", get(players))
        .route("/releases", get(releases))
        .route("/audit", get(audit))
        .route("/catalogs", get(catalogs))
        .route("/catalogs/{catalog_id}", get(catalog))
}

const CATALOGS: [(&str, &str, &str); 8] = [
    ("buildings", "Buildings", include_str!("../../../../packages/content/releases/evil-hunter-1.411/building-registry.json")),
    ("experience", "Experience", include_str!("../../../../packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json")),
    ("gear", "Gear", include_str!("../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json")),
    ("hunter-assets", "Hunter assets", include_str!("../../../../packages/content/releases/evil-hunter-1.411/hunter-assets.json")),
    ("monster-materials", "Monster materials", include_str!("../../../../packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json")),
    ("monsters", "Monsters", include_str!("../../../../packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json")),
    ("world-map", "World map", include_str!("../../../../packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json")),
    ("rebuild-weapon-core", "Rebuild weapon core", include_str!("../../../../packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json")),
];

async fn catalogs(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let catalogs = CATALOGS
        .iter()
        .map(|(id, label, source)| {
            let value: serde_json::Value =
                serde_json::from_str(source).expect("embedded catalog must be valid JSON");
            let mut collections = Vec::new();
            collect_array_collections("", &value, &mut collections);
            serde_json::json!({"id": id, "label": label, "collections": collections})
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({"catalogs": catalogs})).into_response()
}

fn collect_array_collections(
    path: &str,
    value: &serde_json::Value,
    output: &mut Vec<serde_json::Value>,
) {
    match value {
        serde_json::Value::Array(rows) if !path.is_empty() => {
            output.push(serde_json::json!({"id": path, "count": rows.len()}))
        }
        serde_json::Value::Object(fields) => {
            for (key, child) in fields {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                collect_array_collections(&child_path, child, output);
            }
        }
        _ => {}
    }
}

async fn catalog(
    Path(catalog_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let Some((_, _, source)) = CATALOGS.iter().find(|(id, _, _)| *id == catalog_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "admin_catalog_not_found"})),
        )
            .into_response();
    };
    Json(
        serde_json::from_str::<serde_json::Value>(source)
            .expect("embedded catalog must be valid JSON"),
    )
    .into_response()
}

async fn overview(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let Some(pool) = &state.admin_pool else {
        return Json(OverviewResponse {
            service: "evil-hunter-admin",
            status: "ok",
            protocol_version: PROTOCOL_VERSION,
            durable_schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            tick_rate: state.config.simulation.tick_rate,
            players: 0,
            hunters: 0,
            items: 0,
            releases: 0,
        })
        .into_response();
    };
    let row = sqlx::query(
        r#"SELECT
        (SELECT count(*) FROM player_world_state)::bigint AS players,
        (SELECT count(*) FROM player_hunter)::bigint AS hunters,
        (SELECT count(*) FROM economy_item_definition)::bigint AS items,
        (SELECT count(*) FROM content_release)::bigint AS releases"#,
    )
    .fetch_one(pool)
    .await;
    match row {
        Ok(row) => Json(OverviewResponse {
            service: "evil-hunter-admin",
            status: "ok",
            protocol_version: PROTOCOL_VERSION,
            durable_schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            tick_rate: state.config.simulation.tick_rate,
            players: row.get("players"),
            hunters: row.get("hunters"),
            items: row.get("items"),
            releases: row.get("releases"),
        })
        .into_response(),
        Err(_) => unavailable(),
    }
}

async fn items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      r#"SELECT count(*)::bigint
      FROM economy_item_definition e
      LEFT JOIN LATERAL (
        SELECT display_name FROM economy_item_localization x
        WHERE x.release_id = e.release_id AND x.item_id = e.item_id
        ORDER BY (x.locale = 'en') DESC, x.locale LIMIT 1
      ) l ON TRUE
      WHERE e.item_id ILIKE $1 OR COALESCE(e.internal_name, '') ILIKE $1 OR COALESCE(l.display_name, '') ILIKE $1"#,
      r#"SELECT e.item_id AS id,
        COALESCE(l.display_name, e.internal_name, e.item_id) AS name,
        COALESCE(e.item_type, 'unresolved') AS category,
        e.stack_limit, e.town_pays_hunter_gold_per_unit AS sell_value,
        e.release_id, c.lifecycle AS status
      FROM economy_item_definition e
      JOIN content_release c ON c.release_id = e.release_id
      LEFT JOIN LATERAL (
        SELECT display_name FROM economy_item_localization x
        WHERE x.release_id = e.release_id AND x.item_id = e.item_id
        ORDER BY (x.locale = 'en') DESC, x.locale LIMIT 1
      ) l ON TRUE
      WHERE e.item_id ILIKE $1 OR COALESCE(e.internal_name, '') ILIKE $1 OR COALESCE(l.display_name, '') ILIKE $1
      ORDER BY e.item_id LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "name": row.get::<String, _>("name"),
        "category": row.get::<String, _>("category"), "stackLimit": row.get::<Option<i64>, _>("stack_limit"),
        "sellValue": row.get::<Option<i64>, _>("sell_value"), "releaseId": row.get::<String, _>("release_id"),
        "status": row.get::<String, _>("status")
    })).await
}

async fn buildings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM building_definition WHERE building_id ILIKE $1 OR display_name ILIKE $1",
      r#"SELECT building_id AS id, display_name AS name, COALESCE(category, 'unresolved') AS category,
        max_instances, grid_width, grid_height, COALESCE(constructible, false) AS constructible,
        release_id FROM building_definition
      WHERE building_id ILIKE $1 OR display_name ILIKE $1
      ORDER BY building_id LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "name": row.get::<String, _>("name"),
        "category": row.get::<String, _>("category"), "maxInstances": row.get::<i32, _>("max_instances"),
        "gridWidth": row.get::<i32, _>("grid_width"), "gridHeight": row.get::<i32, _>("grid_height"),
        "constructible": row.get::<bool, _>("constructible"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn monsters(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM monster_definition WHERE source_index::text ILIKE $1 OR monster_type::text ILIKE $1",
      r#"SELECT source_index, monster_type, unique_level, race, hp, damage, armor, experience, gold, asset_bundle_id, release_id
      FROM monster_definition WHERE source_index::text ILIKE $1 OR monster_type::text ILIKE $1
      ORDER BY source_index LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<i32, _>("source_index"), "monsterType": row.get::<i32, _>("monster_type"),
        "level": row.get::<i32, _>("unique_level"), "race": row.get::<i32, _>("race"),
        "hp": row.get::<i64, _>("hp"), "damage": row.get::<i64, _>("damage"), "armor": row.get::<i64, _>("armor"),
        "experience": row.get::<i64, _>("experience"), "gold": row.get::<i64, _>("gold"),
        "asset": row.get::<String, _>("asset_bundle_id"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn gear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM gear_definition WHERE gear_kind ILIKE $1 OR display_name ILIKE $1 OR gear_index::text ILIKE $1",
      r#"SELECT gear_kind, gear_index, display_name, description, job, difficulty_group, item_level, visibility, icon_path, release_id
      FROM gear_definition WHERE gear_kind ILIKE $1 OR display_name ILIKE $1 OR gear_index::text ILIKE $1
      ORDER BY gear_kind, gear_index LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": format!("{}:{}", row.get::<String, _>("gear_kind"), row.get::<i32, _>("gear_index")),
        "kind": row.get::<String, _>("gear_kind"), "index": row.get::<i32, _>("gear_index"),
        "name": row.get::<String, _>("display_name"), "description": row.get::<String, _>("description"),
        "job": row.get::<i32, _>("job"), "difficulty": row.get::<i32, _>("difficulty_group"),
        "itemLevel": row.get::<i32, _>("item_level"), "visibility": row.get::<i32, _>("visibility"),
        "icon": row.get::<Option<String>, _>("icon_path"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn rebuild_weapons(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      r#"SELECT count(*)::bigint
      FROM core_game.rebuild_weapon_base w
      JOIN core_game.rebuild_weapon_localization en ON en.release_id = w.release_id AND en.weapon_id = w.weapon_id AND en.locale = 'en'
      JOIN core_game.rebuild_weapon_localization vi ON vi.release_id = w.release_id AND vi.weapon_id = w.weapon_id AND vi.locale = 'vi'
      WHERE w.weapon_id ILIKE $1 OR w.class_name ILIKE $1 OR en.display_name ILIKE $1 OR vi.display_name ILIKE $1"#,
      r#"SELECT w.weapon_id AS id, en.display_name AS name_en, vi.display_name AS name_vi,
        w.class_id, w.class_name, w.weapon_family, w.difficulty, w.unlock_level, w.base_level_cap,
        w.base_power, w.cap_power, w.package_second_value, w.package_source_index,
        w.evidence_state, w.active, v.asset_state, v.family AS visual_family,
        v.inventory_icon_path, v.spine_attachment_path, w.release_id
      FROM core_game.rebuild_weapon_base w
      JOIN core_game.rebuild_weapon_localization en ON en.release_id = w.release_id AND en.weapon_id = w.weapon_id AND en.locale = 'en'
      JOIN core_game.rebuild_weapon_localization vi ON vi.release_id = w.release_id AND vi.weapon_id = w.weapon_id AND vi.locale = 'vi'
      JOIN core_game.rebuild_weapon_visual_binding v ON v.release_id = w.release_id AND v.weapon_id = w.weapon_id
      WHERE w.weapon_id ILIKE $1 OR w.class_name ILIKE $1 OR en.display_name ILIKE $1 OR vi.display_name ILIKE $1
      ORDER BY w.class_id, w.unlock_level LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "nameEn": row.get::<String, _>("name_en"),
        "nameVi": row.get::<String, _>("name_vi"), "classId": row.get::<String, _>("class_id"),
        "className": row.get::<String, _>("class_name"), "family": row.get::<String, _>("weapon_family"),
        "difficulty": row.get::<i32, _>("difficulty"), "unlockLevel": row.get::<i32, _>("unlock_level"),
        "levelCap": row.get::<i32, _>("base_level_cap"), "basePower": row.get::<i32, _>("base_power"),
        "capPower": row.get::<i32, _>("cap_power"), "packageFactor": row.get::<i32, _>("package_second_value"),
        "packageSourceId": row.get::<i32, _>("package_source_index"), "evidence": row.get::<String, _>("evidence_state"),
        "active": row.get::<bool, _>("active"), "assetState": row.get::<String, _>("asset_state"),
        "visualFamily": row.get::<String, _>("visual_family"),
        "inventoryIcon": format!("/evil-admin/legacy-assets/weapon-{}.png", row.get::<i32, _>("package_source_index") + (row.get::<i32, _>("difficulty") - 1).min(6)),
        "attackDamageMin": row.get::<i32, _>("base_power"), "attackDamageMax": row.get::<i32, _>("cap_power"),
        "attackDamageLine": format!("+ {}-{} Attack Damage", row.get::<i32, _>("base_power"), row.get::<i32, _>("cap_power")),
        "spineAttachment": row.get::<String, _>("spine_attachment_path"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn affixes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM core_game.rebuild_affix WHERE affix_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR property_kind ILIKE $1 OR slot_assignment ILIKE $1",
      r#"SELECT affix_id AS id, source_id, name_en, name_vi, property_kind, slot_assignment,
        origin, family, exclusive_group, generation_state, positive_values, negative_values,
        gear_skill_id, evidence_state, release_id
      FROM core_game.rebuild_affix
      WHERE affix_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR property_kind ILIKE $1 OR slot_assignment ILIKE $1
      ORDER BY source_id NULLS FIRST, affix_id LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "sourceId": row.get::<Option<i32>, _>("source_id"),
        "nameEn": row.get::<String, _>("name_en"), "nameVi": row.get::<String, _>("name_vi"),
        "kind": row.get::<String, _>("property_kind"), "slot": row.get::<String, _>("slot_assignment"),
        "origin": row.get::<String, _>("origin"), "family": row.get::<Option<String>, _>("family"),
        "exclusiveGroup": row.get::<Option<String>, _>("exclusive_group"),
        "generationState": row.get::<String, _>("generation_state"),
        "positiveValues": row.get::<serde_json::Value, _>("positive_values"),
        "negativeValues": row.get::<serde_json::Value, _>("negative_values"),
        "gearSkillId": row.get::<Option<i32>, _>("gear_skill_id"), "evidence": row.get::<String, _>("evidence_state"),
        "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn affix_tiers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      r#"SELECT count(*)::bigint FROM core_game.rebuild_affix_tier t
      JOIN core_game.rebuild_affix a ON a.release_id = t.release_id AND a.affix_id = t.affix_id
      WHERE t.tier_id ILIKE $1 OR t.affix_id ILIKE $1 OR a.name_en ILIKE $1 OR a.name_vi ILIKE $1 OR t.difficulty::text ILIKE $1"#,
      r#"SELECT t.tier_id AS id, t.affix_id, a.name_en, a.name_vi, a.slot_assignment,
        a.family, t.difficulty, t.minimum_item_level, t.maximum_item_level,
        t.minimum_value, t.maximum_value, t.value_basis, t.evidence_state, t.release_id
      FROM core_game.rebuild_affix_tier t
      JOIN core_game.rebuild_affix a ON a.release_id = t.release_id AND a.affix_id = t.affix_id
      WHERE t.tier_id ILIKE $1 OR t.affix_id ILIKE $1 OR a.name_en ILIKE $1 OR a.name_vi ILIKE $1 OR t.difficulty::text ILIKE $1
      ORDER BY a.slot_assignment, t.affix_id, t.difficulty LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "affixId": row.get::<String, _>("affix_id"),
        "nameEn": row.get::<String, _>("name_en"), "nameVi": row.get::<String, _>("name_vi"),
        "slot": row.get::<String, _>("slot_assignment"), "family": row.get::<Option<String>, _>("family"),
        "difficulty": row.get::<i32, _>("difficulty"), "minimumItemLevel": row.get::<i32, _>("minimum_item_level"),
        "maximumItemLevel": row.get::<i32, _>("maximum_item_level"), "minimumValue": row.get::<i32, _>("minimum_value"),
        "maximumValue": row.get::<i32, _>("maximum_value"), "valueBasis": row.get::<String, _>("value_basis"),
        "evidence": row.get::<String, _>("evidence_state"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn affix_pools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      r#"SELECT count(*)::bigint FROM core_game.rebuild_weapon_affix_pool p
      JOIN core_game.rebuild_affix a ON a.release_id = p.release_id AND a.affix_id = p.affix_id
      WHERE p.affix_id ILIKE $1 OR a.name_en ILIKE $1 OR a.name_vi ILIKE $1 OR p.slot ILIKE $1 OR p.family ILIKE $1 OR p.exclusive_group ILIKE $1"#,
      r#"SELECT concat(p.weapon_class, ':', p.affix_id, ':', p.slot) AS id, p.weapon_class,
        p.affix_id, a.name_en, a.name_vi, p.slot, p.family, p.exclusive_group,
        p.weight, p.minimum_difficulty, p.maximum_difficulty, p.active, p.evidence_state, p.release_id
      FROM core_game.rebuild_weapon_affix_pool p
      JOIN core_game.rebuild_affix a ON a.release_id = p.release_id AND a.affix_id = p.affix_id
      WHERE p.affix_id ILIKE $1 OR a.name_en ILIKE $1 OR a.name_vi ILIKE $1 OR p.slot ILIKE $1 OR p.family ILIKE $1 OR p.exclusive_group ILIKE $1
      ORDER BY p.slot, p.family LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "weaponClass": row.get::<String, _>("weapon_class"),
        "affixId": row.get::<String, _>("affix_id"), "nameEn": row.get::<String, _>("name_en"),
        "nameVi": row.get::<String, _>("name_vi"), "slot": row.get::<String, _>("slot"),
        "family": row.get::<String, _>("family"), "exclusiveGroup": row.get::<String, _>("exclusive_group"),
        "weight": row.get::<i32, _>("weight"), "minimumDifficulty": row.get::<i32, _>("minimum_difficulty"),
        "maximumDifficulty": row.get::<i32, _>("maximum_difficulty"), "active": row.get::<bool, _>("active"),
        "evidence": row.get::<String, _>("evidence_state"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WikiQuery {
    #[serde(default)]
    search: String,
}

async fn weapon_modifier_wiki(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WikiQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let Some(pool) = &state.admin_pool else {
        return Json(serde_json::json!({"releaseId": null, "entries": []})).into_response();
    };
    let search = format!("%{}%", query.search.trim());
    let rows = sqlx::query(
        r#"SELECT p.affix_id, a.source_id, a.name_en, a.name_vi, a.origin, p.slot,
          p.family, p.exclusive_group, p.weight, p.minimum_difficulty, p.maximum_difficulty,
          a.evidence_state, p.active,
          COALESCE(json_agg(json_build_object(
            'tier', t.difficulty,
            'requiredItemLevel', t.minimum_item_level,
            'minimumValue', t.minimum_value,
            'maximumValue', t.maximum_value,
            'valueBasis', t.value_basis
          ) ORDER BY t.difficulty) FILTER (WHERE t.tier_id IS NOT NULL), '[]'::json) AS tiers,
          p.release_id
        FROM core_game.rebuild_weapon_affix_pool p
        JOIN core_game.rebuild_affix a ON a.release_id = p.release_id AND a.affix_id = p.affix_id
        LEFT JOIN core_game.rebuild_affix_tier t ON t.release_id = p.release_id AND t.affix_id = p.affix_id
        WHERE p.active AND (p.affix_id ILIKE $1 OR a.name_en ILIKE $1 OR a.name_vi ILIKE $1 OR p.family ILIKE $1 OR p.slot ILIKE $1)
        GROUP BY p.affix_id, a.source_id, a.name_en, a.name_vi, a.origin, p.slot,
          p.family, p.exclusive_group, p.weight, p.minimum_difficulty, p.maximum_difficulty,
          a.evidence_state, p.active, p.release_id
        ORDER BY p.slot, p.family"#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await;
    match rows {
        Ok(rows) => {
            let entries = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": row.get::<String, _>("affix_id"),
                        "sourceId": row.get::<Option<i32>, _>("source_id"),
                        "nameEn": row.get::<String, _>("name_en"),
                        "nameVi": row.get::<String, _>("name_vi"),
                        "origin": row.get::<String, _>("origin"),
                        "slot": row.get::<String, _>("slot"),
                        "family": row.get::<String, _>("family"),
                        "exclusiveGroup": row.get::<String, _>("exclusive_group"),
                        "weight": row.get::<i32, _>("weight"),
                        "minimumDifficulty": row.get::<i32, _>("minimum_difficulty"),
                        "maximumDifficulty": row.get::<i32, _>("maximum_difficulty"),
                        "evidence": row.get::<String, _>("evidence_state"),
                        "active": row.get::<bool, _>("active"),
                        "tiers": row.get::<serde_json::Value, _>("tiers"),
                        "releaseId": row.get::<String, _>("release_id")
                    })
                })
                .collect::<Vec<_>>();
            let release_id = entries
                .first()
                .and_then(|entry| entry.get("releaseId"))
                .cloned();
            Json(serde_json::json!({
                "releaseId": release_id,
                "tierPolicy": "item-level eligibility; inclusive integer roll",
                "entries": entries,
            }))
            .into_response()
        }
        Err(_) => unavailable(),
    }
}

async fn virtues(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM core_game.rebuild_virtue_effect WHERE virtue_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR description_en ILIKE $1 OR description_vi ILIKE $1",
      r#"SELECT virtue_id AS id, source_id, name_en, name_vi, description_en, description_vi,
        threshold_values, secondary_value, tertiary_value, evidence_state, release_id
      FROM core_game.rebuild_virtue_effect
      WHERE virtue_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR description_en ILIKE $1 OR description_vi ILIKE $1
      ORDER BY source_id LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "sourceId": row.get::<i32, _>("source_id"),
        "nameEn": row.get::<String, _>("name_en"), "nameVi": row.get::<String, _>("name_vi"),
        "descriptionEn": row.get::<String, _>("description_en"), "descriptionVi": row.get::<String, _>("description_vi"),
        "thresholds": row.get::<serde_json::Value, _>("threshold_values"),
        "secondaryValue": row.get::<f64, _>("secondary_value"), "tertiaryValue": row.get::<f64, _>("tertiary_value"),
        "evidence": row.get::<String, _>("evidence_state"), "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn collection_sets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM core_game.rebuild_collection_set WHERE set_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR effect_state ILIKE $1",
      r#"SELECT set_id AS id, source_id, name_en, name_vi, special_item_ids, option_type,
        option_value, visible, effect_state, evidence_state, release_id
      FROM core_game.rebuild_collection_set
      WHERE set_id ILIKE $1 OR name_en ILIKE $1 OR name_vi ILIKE $1 OR effect_state ILIKE $1
      ORDER BY source_id LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "sourceId": row.get::<i32, _>("source_id"),
        "nameEn": row.get::<String, _>("name_en"), "nameVi": row.get::<String, _>("name_vi"),
        "itemIds": row.get::<serde_json::Value, _>("special_item_ids"), "optionType": row.get::<i32, _>("option_type"),
        "optionValue": row.get::<f64, _>("option_value"), "visible": row.get::<i32, _>("visible"),
        "effectState": row.get::<String, _>("effect_state"), "evidence": row.get::<String, _>("evidence_state"),
        "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn consumables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM consumable_definition WHERE consumable_index::text ILIKE $1 OR consumable_type::text ILIKE $1",
      r#"SELECT consumable_index, consumable_type, max_level, cooldown_ms, release_id
      FROM consumable_definition WHERE consumable_index::text ILIKE $1 OR consumable_type::text ILIKE $1
      ORDER BY consumable_index LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<i32, _>("consumable_index"), "type": row.get::<i32, _>("consumable_type"),
        "maxLevel": row.get::<i32, _>("max_level"), "cooldownMs": row.get::<i64, _>("cooldown_ms"),
        "releaseId": row.get::<String, _>("release_id")
    })).await
}

async fn hunters(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM player_hunter WHERE display_name ILIKE $1 OR class_id ILIKE $1 OR rarity_id ILIKE $1 OR player_token::text ILIKE $1",
      r#"SELECT h.player_token::text AS player_id, h.hunter_id,
        h.display_name, h.roster_state, h.class_id, COALESCE(c.display_name, h.class_id) AS class_name,
        h.rarity_id, COALESCE(r.display_name, h.rarity_id) AS rarity_name, h.level, h.xp,
        h.current_hp, h.max_hp, h.gold, h.action_state
      FROM player_hunter h
      LEFT JOIN hunter_class_definition c ON c.release_id = h.content_release_id AND c.class_id = h.class_id
      LEFT JOIN hunter_rarity_definition r ON r.release_id = h.content_release_id AND r.rarity_id = h.rarity_id
      WHERE h.display_name ILIKE $1 OR h.class_id ILIKE $1 OR h.rarity_id ILIKE $1 OR h.player_token::text ILIKE $1
      ORDER BY h.player_token, h.roster_state, h.roster_position LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "playerId": row.get::<String, _>("player_id"), "hunterId": row.get::<i64, _>("hunter_id"),
        "name": row.get::<String, _>("display_name"), "rosterState": row.get::<String, _>("roster_state"),
        "classId": row.get::<String, _>("class_id"), "className": row.get::<String, _>("class_name"),
        "rarityId": row.get::<String, _>("rarity_id"), "rarityName": row.get::<String, _>("rarity_name"),
        "level": row.get::<i32, _>("level"), "xp": row.get::<i64, _>("xp"),
        "currentHp": row.get::<i64, _>("current_hp"), "maxHp": row.get::<i64, _>("max_hp"),
        "gold": row.get::<i64, _>("gold"), "actionState": row.get::<String, _>("action_state")
    })).await
}

async fn players(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM player_world_state WHERE player_token::text ILIKE $1",
      r#"SELECT p.player_token::text AS id, p.revision,
        p.created_at::text AS created_at, p.updated_at::text AS updated_at,
        COALESCE(t.gold, 0) AS town_gold, count(h.hunter_id)::bigint AS hunter_count
      FROM player_world_state p
      LEFT JOIN town t ON t.player_token = p.player_token
      LEFT JOIN player_hunter h ON h.player_token = p.player_token
      WHERE p.player_token::text ILIKE $1
      GROUP BY p.player_token, p.revision, p.created_at, p.updated_at, t.gold
      ORDER BY p.updated_at DESC LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "revision": row.get::<i64, _>("revision"),
        "createdAt": row.get::<String, _>("created_at"), "updatedAt": row.get::<String, _>("updated_at"),
        "townGold": row.get::<i64, _>("town_gold"), "hunterCount": row.get::<i64, _>("hunter_count")
    })).await
}

async fn releases(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM (SELECT release_id FROM content_release UNION ALL SELECT release_id FROM hunter_content_release) releases WHERE release_id ILIKE $1",
      r#"SELECT * FROM (SELECT release_id AS id, lifecycle AS status, 'gameplay' AS kind,
        created_at::text AS created_at FROM content_release
      UNION ALL
      SELECT release_id AS id, status, 'hunter' AS kind, created_at::text AS created_at FROM hunter_content_release) releases
      WHERE id ILIKE $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "id": row.get::<String, _>("id"), "status": row.get::<String, _>("status"),
        "kind": row.get::<String, _>("kind"), "createdAt": row.get::<String, _>("created_at")
    })).await
}

async fn audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageQuery>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    paged_rows(&state, &page,
      "SELECT count(*)::bigint FROM (SELECT player_token::text AS player_id, command_type AS action FROM command_ledger UNION ALL SELECT player_token::text, reason FROM reward_ledger) events WHERE player_id ILIKE $1 OR action ILIKE $1",
      r#"SELECT * FROM (SELECT created_at::text AS created_at, player_token::text AS player_id,
        'command' AS kind, command_type AS action FROM command_ledger
      UNION ALL
      SELECT created_at::text, player_token::text, 'reward', reason FROM reward_ledger) events
      WHERE player_id ILIKE $1 OR action ILIKE $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#, |row| serde_json::json!({
        "createdAt": row.get::<String, _>("created_at"), "playerId": row.get::<String, _>("player_id"),
        "kind": row.get::<String, _>("kind"), "action": row.get::<String, _>("action")
    })).await
}

async fn paged_rows<F>(
    state: &AppState,
    query: &PageQuery,
    count_sql: &str,
    data_sql: &str,
    project: F,
) -> Response
where
    F: Fn(&sqlx::postgres::PgRow) -> serde_json::Value,
{
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(10, 100);
    let search = format!("%{}%", query.search.trim());
    let Some(pool) = &state.admin_pool else {
        return Json(PageResponse {
            data: Vec::new(),
            page,
            page_size,
            total: 0,
            total_pages: 0,
        })
        .into_response();
    };
    let total = sqlx::query_scalar::<_, i64>(count_sql)
        .bind(&search)
        .fetch_one(pool)
        .await;
    let rows = sqlx::query(data_sql)
        .bind(&search)
        .bind(page_size)
        .bind((page - 1) * page_size)
        .fetch_all(pool)
        .await;
    match (total, rows) {
        (Ok(total), Ok(rows)) => Json(PageResponse {
            data: rows.iter().map(project).collect(),
            page,
            page_size,
            total,
            total_pages: if total == 0 {
                0
            } else {
                (total + page_size - 1) / page_size
            },
        })
        .into_response(),
        _ => unavailable(),
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            "Basic realm=\"Evil Hunter Admin\"",
        )],
        Json(serde_json::json!({"error": "admin_auth_required"})),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({"error": "admin_data_unavailable"})),
    )
        .into_response()
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(credentials) = std::str::from_utf8(&decoded) else {
        return false;
    };
    let Some((username, password)) = credentials.split_once(':') else {
        return false;
    };
    constant_time_eq(username.as_bytes(), state.config.admin.username.as_bytes())
        & constant_time_eq(password.as_bytes(), state.config.admin.password.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app_for_test, config::AppConfig};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn admin_requires_basic_authentication() {
        let response = app_for_test(AppConfig::for_test())
            .oneshot(Request::get("/admin/overview").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
    }

    #[tokio::test]
    async fn admin_returns_overview_for_valid_credentials() {
        let credentials = STANDARD.encode("admin:test-password");
        let response = app_for_test(AppConfig::for_test())
            .oneshot(
                Request::get("/admin/overview")
                    .header(header::AUTHORIZATION, format!("Basic {credentials}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16_384).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["service"], "evil-hunter-admin");
    }

    #[tokio::test]
    async fn weapon_core_routes_are_registered_and_authenticated() {
        let credentials = STANDARD.encode("admin:test-password");
        for path in [
            "rebuild-weapons",
            "affixes",
            "affix-tiers",
            "affix-pools",
            "wiki/weapon-modifiers",
            "virtues",
            "collection-sets",
        ] {
            let response = app_for_test(AppConfig::for_test())
                .oneshot(
                    Request::get(format!("/admin/{path}"))
                        .header(header::AUTHORIZATION, format!("Basic {credentials}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "route {path}");
            let body = to_bytes(response.into_body(), 16_384).await.unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            if path == "wiki/weapon-modifiers" {
                assert_eq!(json["entries"], serde_json::json!([]));
            } else {
                assert_eq!(json["data"], serde_json::json!([]));
            }
        }
    }
}
