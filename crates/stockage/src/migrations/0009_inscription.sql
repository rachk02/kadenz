-- Phase 2, jalon F4 : codes d'inscription des postes (décision 36 ;
-- docs/spec-synchronisation.md, section 7). Seule l'empreinte SHA-256 du
-- secret est gardée ; un code sert une fois, expire au bout de 24 heures et
-- peut être annulé. Heures en UTC (décision 37), au format AAAA-MM-JJTHH:MM:SSZ.
CREATE TABLE code_inscription (
    id           INTEGER PRIMARY KEY,
    secret_hache TEXT NOT NULL UNIQUE,
    nom_poste    TEXT NOT NULL,
    cree_le      TEXT NOT NULL,
    expire_le    TEXT NOT NULL,
    servi_le     TEXT,
    annule_le    TEXT,
    -- Empreinte du certificat du poste inscrit avec ce code.
    poste        TEXT
) STRICT;
