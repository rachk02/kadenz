-- Phase 2, jalon F2 : postes inscrits auprès du serveur (docs/spec-synchronisation.md,
-- section 7). Un poste est reconnu par l'empreinte SHA-256 de son certificat
-- TLS ; l'inscription par code (F4) remplit cette table, le retrait d'un poste
-- l'empêche de se connecter sans effacer la trace de son inscription.
CREATE TABLE poste_inscrit (
    empreinte  TEXT PRIMARY KEY,
    nom        TEXT NOT NULL,
    inscrit_le TEXT NOT NULL,
    retire_le  TEXT
) STRICT;
