-- Versioned world-map content. Simulation code owns behavior; PostgreSQL owns
-- map identity, tuning, geometry, and asset bindings for the active release.
CREATE TABLE world_map_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    map_id TEXT NOT NULL CHECK (btrim(map_id) <> ''),
    area INTEGER NOT NULL CHECK (area >= 0),
    monster_tier INTEGER NOT NULL CHECK (monster_tier > 0),
    map_asset_id TEXT NOT NULL CHECK (btrim(map_asset_id) <> ''),
    min_x INTEGER NOT NULL,
    max_x INTEGER NOT NULL,
    min_y INTEGER NOT NULL,
    max_y INTEGER NOT NULL,
    PRIMARY KEY (release_id, map_id),
    CHECK (min_x <= max_x AND min_y <= max_y)
);

CREATE TABLE world_map_density_definition (
    release_id TEXT NOT NULL,
    map_id TEXT NOT NULL,
    density_level INTEGER NOT NULL CHECK (density_level BETWEEN 1 AND 3),
    spawn_count INTEGER NOT NULL CHECK (spawn_count >= 0),
    PRIMARY KEY (release_id, map_id, density_level),
    FOREIGN KEY (release_id, map_id)
        REFERENCES world_map_definition(release_id, map_id) ON DELETE CASCADE
);

CREATE TABLE world_map_entry_waypoint (
    release_id TEXT NOT NULL,
    map_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 2),
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    PRIMARY KEY (release_id, map_id, ordinal),
    FOREIGN KEY (release_id, map_id)
        REFERENCES world_map_definition(release_id, map_id) ON DELETE CASCADE
);

INSERT INTO world_map_definition
    (release_id, map_id, area, monster_tier, map_asset_id, min_x, max_x, min_y, max_y)
VALUES
    ('evil-hunter-1.411.buildings-v1', 'map_new01', 0, 1,
     '/content/releases/visible-world-v1/maps/map_new01.png', 320, 1030, 500, 1000),
    ('evil-hunter-1.411.buildings-v1', 'background_08', 1, 2,
     '/content/releases/visible-world-v1/village/background/background_08__1530.png', 1080, 1760, 1080, 1430),
    ('evil-hunter-1.411.buildings-v1', 'background_11', 2, 3,
     '/content/releases/visible-world-v1/village/background/background_11__1508.png', 2220, 2860, 500, 1030);

INSERT INTO world_map_density_definition (release_id, map_id, density_level, spawn_count)
SELECT 'evil-hunter-1.411.buildings-v1', map_id, density_level, spawn_count
FROM (VALUES
    ('map_new01', 1, 3), ('map_new01', 2, 6), ('map_new01', 3, 9),
    ('background_08', 1, 3), ('background_08', 2, 6), ('background_08', 3, 9),
    ('background_11', 1, 3), ('background_11', 2, 6), ('background_11', 3, 9)
) AS rows(map_id, density_level, spawn_count);

INSERT INTO world_map_entry_waypoint (release_id, map_id, ordinal, x, y)
SELECT 'evil-hunter-1.411.buildings-v1', map_id, ordinal, x, y
FROM (VALUES
    ('map_new01', 0, 1410, 690), ('map_new01', 1, 1356, 800), ('map_new01', 2, 1273, 800),
    ('background_08', 0, 1410, 690), ('background_08', 1, 1356, 800), ('background_08', 2, 1356, 861),
    ('background_11', 0, 1957, 809), ('background_11', 1, 2043, 724), ('background_11', 2, 2127, 724)
) AS rows(map_id, ordinal, x, y);
