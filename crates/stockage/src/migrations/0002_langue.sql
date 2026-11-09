-- Langue de l'interface, choisie par compte (décision du 2026-11-09). Elle ne vaut
-- que pour l'interface : les documents remis aux clients restent en français.
-- Codes ISO 639-1 : français, anglais, arabe, japonais, chinois, turc, espagnol.
ALTER TABLE utilisateur ADD COLUMN langue TEXT NOT NULL DEFAULT 'fr'
    CHECK (langue IN ('fr', 'en', 'ar', 'ja', 'zh', 'tr', 'es'));
