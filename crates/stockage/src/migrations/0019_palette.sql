-- Palette du mode sombre choisie par compte (décision de l'utilisateur du
-- 2026-11-18) : Graphite, Ardoise ou Anthracite chaud (par défaut). Comme le
-- mode (migration 0018), propre à chaque base, jamais répliquée.
ALTER TABLE utilisateur ADD COLUMN palette TEXT NOT NULL DEFAULT 'anthracite'
    CHECK (palette IN ('graphite', 'ardoise', 'anthracite'));
-- 1 quand le compte a fait son propre choix dans les Paramètres ; 0 : le compte
-- suit le réglage du poste (fichier affichage.json du dossier des données,
-- choisi au premier lancement).
ALTER TABLE utilisateur ADD COLUMN affichage_personnel INTEGER NOT NULL DEFAULT 0
    CHECK (affichage_personnel IN (0, 1));
