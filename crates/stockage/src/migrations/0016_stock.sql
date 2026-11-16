-- Phase 2, étape 2 : gestion de stock (docs/spec-stock.md, jalon S1 ;
-- décisions 64 à 67).

-- Suivi en stock d'un produit, décoché par défaut (un service le reste).
ALTER TABLE produit ADD COLUMN suivi_stock INTEGER NOT NULL DEFAULT 0 CHECK (suivi_stock IN (0, 1));

-- Mouvements de stock, en ajout seul : créés par le serveur en appliquant la
-- réception d'un bon de commande, la confirmation d'un bordereau, un
-- ajustement ou un inventaire ; jamais sur la copie d'un poste, qui les reçoit
-- du serveur. Chaque mouvement garde le stock du produit après lui : le stock
-- du moment est celui du dernier mouvement (rang le plus grand), le stock à
-- une date celui du dernier mouvement de ce jour ou d'avant.
CREATE TABLE mouvement_stock (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    uid            TEXT,
    -- Ordre des mouvements sur le serveur, recopié tel quel sur les postes.
    rang           INTEGER NOT NULL,
    produit_id     INTEGER NOT NULL REFERENCES produit (id),
    nature         TEXT NOT NULL CHECK (nature IN ('reception', 'livraison', 'ajustement', 'inventaire')),
    -- Jour du serveur, dans le fuseau de l'entreprise (AAAA-MM-JJ).
    date           TEXT NOT NULL,
    -- Heure technique du serveur.
    heure          TEXT NOT NULL,
    -- Quantité signée en millièmes et valeur signée en FCFA.
    quantite       INTEGER NOT NULL,
    valeur         INTEGER NOT NULL,
    quantite_apres INTEGER NOT NULL,
    valeur_apres   INTEGER NOT NULL,
    -- Dernier coût unitaire connu après le mouvement (FCFA par unité).
    cout_apres     INTEGER NOT NULL,
    document_id    INTEGER REFERENCES document (id),
    -- Rang de la ligne du document (0 pour la première), s'il y a un document.
    ligne          INTEGER,
    motif          TEXT NOT NULL DEFAULT '',
    utilisateur_id INTEGER REFERENCES utilisateur (id)
) STRICT;
CREATE UNIQUE INDEX mouvement_stock_uid ON mouvement_stock (uid);
CREATE INDEX mouvement_stock_produit ON mouvement_stock (produit_id, rang);
CREATE INDEX mouvement_stock_date ON mouvement_stock (date, rang);

CREATE TRIGGER mouvement_stock_suivi_creation AFTER INSERT ON mouvement_stock BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('mouvement', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER mouvement_stock_suivi_modification AFTER UPDATE ON mouvement_stock
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('mouvement', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER mouvement_stock_suivi_suppression AFTER DELETE ON mouvement_stock BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('mouvement', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Inventaire en cours sur un poste (spec, 3.8) : quantités comptées gardées
-- jusqu'à la validation, propres à la base qui les tient, jamais répliquées.
CREATE TABLE comptage_stock (
    produit_id       INTEGER PRIMARY KEY REFERENCES produit (id) ON DELETE CASCADE,
    quantite_comptee INTEGER,
    cout_unitaire    INTEGER
) STRICT;

-- Nouveaux droits (spec, 3.7), accordés à l'Admin et au Comptable dans une
-- matrice déjà enregistrée ; sans matrice enregistrée, celle du cahier les
-- porte déjà.
UPDATE parametre_systeme
SET valeur = json_insert(valeur, '$.admin[#]', 'ajuster_stock', '$.admin[#]', 'voir_valeur_stock')
WHERE cle = 'droits.matrice' AND json_valid(valeur) AND json_type(valeur, '$.admin') = 'array';
UPDATE parametre_systeme
SET valeur = json_insert(valeur, '$.comptable[#]', 'ajuster_stock', '$.comptable[#]', 'voir_valeur_stock')
WHERE cle = 'droits.matrice' AND json_valid(valeur) AND json_type(valeur, '$.comptable') = 'array';
