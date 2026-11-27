-- Révocation à distance (décision 80, docs/spec-revocation-licences.md) :
-- identifiant de l'ordinateur donné par le poste à son inscription (vide pour
-- un poste ancien), dernier contact du poste avec le serveur.
ALTER TABLE poste_inscrit ADD COLUMN ordinateur TEXT NOT NULL DEFAULT '';
ALTER TABLE poste_inscrit ADD COLUMN vu_le TEXT;
