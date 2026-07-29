-- The runtime protocol uses stable class-scoped IDs while the recovered table
-- keeps numeric basic:* IDs. Persist both as explicit aliases of the same rows.
INSERT INTO hunter_skill_definition (
    release_id, skill_id, class_id, display_name, icon_path, animation_name,
    evidence_confidence, semantics_status, source_kind, source_index,
    description, detail_description, max_level, sub_job, third_job, fourth_job,
    source_parameters
)
SELECT
    'migration.hunter-demo-v1', aliases.skill_id, source.class_id,
    source.display_name, source.icon_path, source.animation_name,
    source.evidence_confidence, source.semantics_status, NULL, NULL,
    source.description, source.detail_description, source.max_level,
    source.sub_job, source.third_job, source.fourth_job, source.source_parameters
FROM (VALUES
    ('skill_h1_01', 'basic:0'), ('skill_h1_02', 'basic:1'),
    ('skill_h2_01', 'basic:2'), ('skill_h2_02', 'basic:3'),
    ('skill_h3_01', 'basic:4'), ('skill_h3_02', 'basic:5'),
    ('skill_h4_01', 'basic:6'), ('skill_h4_02', 'basic:7'),
    ('skill_h5_01', 'basic:8'), ('skill_h5_02', 'basic:9')
) AS aliases(skill_id, source_skill_id)
JOIN hunter_skill_definition AS source
  ON source.release_id = 'evil-hunter-1.411.hunter-info-v1'
 AND source.skill_id = aliases.source_skill_id
ON CONFLICT (release_id, skill_id) DO NOTHING;
