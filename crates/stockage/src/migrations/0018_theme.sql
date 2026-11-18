-- Mode d'affichage choisi par compte (décision de l'utilisateur du 2026-11-18) :
-- comme Windows (par défaut), clair ou sombre. Préférence propre à chaque base :
-- jamais répliquée du serveur vers les postes, chaque poste garde la sienne.
ALTER TABLE utilisateur ADD COLUMN theme TEXT NOT NULL DEFAULT 'systeme'
    CHECK (theme IN ('systeme', 'clair', 'sombre'));
