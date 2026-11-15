-- Phase 2, jalon F8 : case « accès à distance » de chaque poste inscrit
-- (décision 57), décochée par défaut. Seuls les postes cochés passent par le
-- relais.

ALTER TABLE poste_inscrit ADD COLUMN distance INTEGER NOT NULL DEFAULT 0 CHECK (distance IN (0, 1));
