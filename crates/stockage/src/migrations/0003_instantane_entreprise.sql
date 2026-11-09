-- Réimpression identique à l'original (décision du 2026-11-09, section 12) : à la
-- validation, les données de l'entreprise, ses logos, les textes du document et la
-- couleur sont figés dans un instantané, référencé par le document. Un instantané
-- identique au précédent est réutilisé : les logos ne sont pas recopiés à chaque
-- validation.
CREATE TABLE instantane_entreprise (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    donnees TEXT NOT NULL CHECK (json_valid(donnees)),
    logo    BLOB,
    logo2   BLOB
) STRICT;

ALTER TABLE document ADD COLUMN instantane_entreprise_id INTEGER REFERENCES instantane_entreprise (id);
