CREATE TABLE hunter_visual_component_definition (
    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,
    component_id TEXT NOT NULL,
    component_kind TEXT NOT NULL CHECK (component_kind IN ('class_base', 'appearance', 'costume', 'hat', 'weapon')),
    class_id TEXT,
    spine_skin_name TEXT NOT NULL,
    evidence_confidence TEXT NOT NULL CHECK (evidence_confidence IN ('confirmed', 'strongly_inferred', 'tentative', 'unknown')),
    semantics_status TEXT NOT NULL CHECK (semantics_status IN ('resolved', 'visual_only', 'unresolved')),
    PRIMARY KEY (release_id, component_id),
    UNIQUE (release_id, spine_skin_name),
    FOREIGN KEY (release_id, class_id) REFERENCES hunter_class_definition(release_id, class_id)
);

CREATE TABLE player_hunter_visual_component (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    component_kind TEXT NOT NULL CHECK (component_kind IN ('class_base', 'appearance', 'costume', 'hat', 'weapon')),
    release_id TEXT NOT NULL,
    component_id TEXT NOT NULL,
    equipped_order SMALLINT NOT NULL CHECK (equipped_order BETWEEN 0 AND 15),
    PRIMARY KEY (player_token, hunter_id, component_kind),
    UNIQUE (player_token, hunter_id, equipped_order),
    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, component_id) REFERENCES hunter_visual_component_definition(release_id, component_id)
);

INSERT INTO hunter_visual_component_definition
    (release_id, component_id, component_kind, class_id, spine_skin_name, evidence_confidence, semantics_status)
VALUES
    ('migration.hunter-demo-v1','class:all_h1','class_base','h1','All_h1','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h1_duallist','class_base','h1','All_h1_duallist','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h2_executor','class_base','h2','All_h2_executor','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h2_templer','class_base','h2','All_h2_templer','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h3_mistic','class_base','h3','All_h3_mistic','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h4','class_base','h4','All_h4','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h4_darkload','class_base','h4','All_h4_darkload','confirmed','visual_only'),
    ('migration.hunter-demo-v1','class:all_h5_concentrate','class_base','h5','All_h5_concentrate','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_f_01','appearance',NULL,'hunter_f_01','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_m_21','appearance',NULL,'hunter_m_21','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_f_41','appearance',NULL,'hunter_f_41','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_m_61','appearance',NULL,'hunter_m_61','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_f_81','appearance',NULL,'hunter_f_81','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_m_101','appearance',NULL,'hunter_m_101','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_f_111','appearance',NULL,'hunter_f_111','confirmed','visual_only'),
    ('migration.hunter-demo-v1','appearance:hunter_m_117','appearance',NULL,'hunter_m_117','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h4_01','costume','h4','costum_h4_01','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h1_02','costume','h1','costum_h1_02','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h3_03','costume','h3','costum_h3_03','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h2_04','costume','h2','costum_h2_04','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h5_05','costume','h5','costum_h5_05','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h1_06','costume','h1','costum_h1_06','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h2_07','costume','h2','costum_h2_07','confirmed','visual_only'),
    ('migration.hunter-demo-v1','costume:h4_08','costume','h4','costum_h4_08','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:01','hat',NULL,'hat_01','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:02','hat',NULL,'hat_02','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:03','hat',NULL,'hat_03','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:04','hat',NULL,'hat_04','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:05','hat',NULL,'hat_05','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:06','hat',NULL,'hat_06','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:07','hat',NULL,'hat_07','confirmed','visual_only'),
    ('migration.hunter-demo-v1','hat:08','hat',NULL,'hat_08','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h4_a_01','weapon','h4','weapon_h4_a_01','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h1_b_02','weapon','h1','weapon_h1_b_02','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h3_c_03','weapon','h3','weapon_h3_c_03','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h2_d_04','weapon','h2','weapon_h2_d_04','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h5_a_04','weapon','h5','weapon_h5_a_04','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h1_c_03','weapon','h1','weapon_h1_c_03','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h2_b_01','weapon','h2','weapon_h2_b_01','confirmed','visual_only'),
    ('migration.hunter-demo-v1','weapon:h4_c_02','weapon','h4','weapon_h4_c_02','confirmed','visual_only');

UPDATE player_hunter
SET portrait_asset_id = CASE hunter_id
    WHEN 7 THEN '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_111__5928.png'
    WHEN 8 THEN '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_117__3163.png'
    ELSE portrait_asset_id
END
WHERE player_token = '00000000-0000-4000-8000-00000000a001';

INSERT INTO player_hunter_visual_component
    (player_token, hunter_id, component_kind, release_id, component_id, equipped_order)
SELECT seed.player_token::uuid, seed.hunter_id, seed.component_kind,
       seed.release_id, seed.component_id, seed.equipped_order
FROM (VALUES
    ('00000000-0000-4000-8000-00000000a001',1,'class_base','migration.hunter-demo-v1','class:all_h4_darkload',0),
    ('00000000-0000-4000-8000-00000000a001',1,'appearance','migration.hunter-demo-v1','appearance:hunter_f_01',1),
    ('00000000-0000-4000-8000-00000000a001',1,'costume','migration.hunter-demo-v1','costume:h4_01',2),
    ('00000000-0000-4000-8000-00000000a001',1,'hat','migration.hunter-demo-v1','hat:01',3),
    ('00000000-0000-4000-8000-00000000a001',1,'weapon','migration.hunter-demo-v1','weapon:h4_a_01',4),
    ('00000000-0000-4000-8000-00000000a001',2,'class_base','migration.hunter-demo-v1','class:all_h1_duallist',0),
    ('00000000-0000-4000-8000-00000000a001',2,'appearance','migration.hunter-demo-v1','appearance:hunter_m_21',1),
    ('00000000-0000-4000-8000-00000000a001',2,'costume','migration.hunter-demo-v1','costume:h1_02',2),
    ('00000000-0000-4000-8000-00000000a001',2,'hat','migration.hunter-demo-v1','hat:02',3),
    ('00000000-0000-4000-8000-00000000a001',2,'weapon','migration.hunter-demo-v1','weapon:h1_b_02',4),
    ('00000000-0000-4000-8000-00000000a001',3,'class_base','migration.hunter-demo-v1','class:all_h3_mistic',0),
    ('00000000-0000-4000-8000-00000000a001',3,'appearance','migration.hunter-demo-v1','appearance:hunter_f_41',1),
    ('00000000-0000-4000-8000-00000000a001',3,'costume','migration.hunter-demo-v1','costume:h3_03',2),
    ('00000000-0000-4000-8000-00000000a001',3,'hat','migration.hunter-demo-v1','hat:03',3),
    ('00000000-0000-4000-8000-00000000a001',3,'weapon','migration.hunter-demo-v1','weapon:h3_c_03',4),
    ('00000000-0000-4000-8000-00000000a001',4,'class_base','migration.hunter-demo-v1','class:all_h2_executor',0),
    ('00000000-0000-4000-8000-00000000a001',4,'appearance','migration.hunter-demo-v1','appearance:hunter_m_61',1),
    ('00000000-0000-4000-8000-00000000a001',4,'costume','migration.hunter-demo-v1','costume:h2_04',2),
    ('00000000-0000-4000-8000-00000000a001',4,'hat','migration.hunter-demo-v1','hat:04',3),
    ('00000000-0000-4000-8000-00000000a001',4,'weapon','migration.hunter-demo-v1','weapon:h2_d_04',4),
    ('00000000-0000-4000-8000-00000000a001',5,'class_base','migration.hunter-demo-v1','class:all_h5_concentrate',0),
    ('00000000-0000-4000-8000-00000000a001',5,'appearance','migration.hunter-demo-v1','appearance:hunter_f_81',1),
    ('00000000-0000-4000-8000-00000000a001',5,'costume','migration.hunter-demo-v1','costume:h5_05',2),
    ('00000000-0000-4000-8000-00000000a001',5,'hat','migration.hunter-demo-v1','hat:05',3),
    ('00000000-0000-4000-8000-00000000a001',5,'weapon','migration.hunter-demo-v1','weapon:h5_a_04',4),
    ('00000000-0000-4000-8000-00000000a001',6,'class_base','migration.hunter-demo-v1','class:all_h1',0),
    ('00000000-0000-4000-8000-00000000a001',6,'appearance','migration.hunter-demo-v1','appearance:hunter_m_101',1),
    ('00000000-0000-4000-8000-00000000a001',6,'costume','migration.hunter-demo-v1','costume:h1_06',2),
    ('00000000-0000-4000-8000-00000000a001',6,'hat','migration.hunter-demo-v1','hat:06',3),
    ('00000000-0000-4000-8000-00000000a001',6,'weapon','migration.hunter-demo-v1','weapon:h1_c_03',4),
    ('00000000-0000-4000-8000-00000000a001',7,'class_base','migration.hunter-demo-v1','class:all_h2_templer',0),
    ('00000000-0000-4000-8000-00000000a001',7,'appearance','migration.hunter-demo-v1','appearance:hunter_f_111',1),
    ('00000000-0000-4000-8000-00000000a001',7,'costume','migration.hunter-demo-v1','costume:h2_07',2),
    ('00000000-0000-4000-8000-00000000a001',7,'hat','migration.hunter-demo-v1','hat:07',3),
    ('00000000-0000-4000-8000-00000000a001',7,'weapon','migration.hunter-demo-v1','weapon:h2_b_01',4),
    ('00000000-0000-4000-8000-00000000a001',8,'class_base','migration.hunter-demo-v1','class:all_h4',0),
    ('00000000-0000-4000-8000-00000000a001',8,'appearance','migration.hunter-demo-v1','appearance:hunter_m_117',1),
    ('00000000-0000-4000-8000-00000000a001',8,'costume','migration.hunter-demo-v1','costume:h4_08',2),
    ('00000000-0000-4000-8000-00000000a001',8,'hat','migration.hunter-demo-v1','hat:08',3),
    ('00000000-0000-4000-8000-00000000a001',8,'weapon','migration.hunter-demo-v1','weapon:h4_c_02',4)
) AS seed(player_token, hunter_id, component_kind, release_id, component_id, equipped_order)
JOIN player_hunter hunter
  ON hunter.player_token = seed.player_token::uuid
 AND hunter.hunter_id = seed.hunter_id;
