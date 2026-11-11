-- Affichage choisi par compte (décisions de l'utilisateur du 2026-11-11) :
-- navigation par bandeau en haut (par défaut) ou par barre latérale repliable,
-- et animations de l'interface, actives par défaut.
ALTER TABLE utilisateur ADD COLUMN navigation TEXT NOT NULL DEFAULT 'bandeau'
    CHECK (navigation IN ('bandeau', 'barre'));
ALTER TABLE utilisateur ADD COLUMN animations INTEGER NOT NULL DEFAULT 1
    CHECK (animations IN (0, 1));
