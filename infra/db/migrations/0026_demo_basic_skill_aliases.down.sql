DELETE FROM hunter_skill_definition
WHERE release_id = 'migration.hunter-demo-v1'
  AND skill_id IN (
    'skill_h1_01', 'skill_h1_02', 'skill_h2_01', 'skill_h2_02',
    'skill_h3_01', 'skill_h3_02', 'skill_h4_01', 'skill_h4_02',
    'skill_h5_01', 'skill_h5_02'
  );
