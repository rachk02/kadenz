-- Phase 2, jalon F1 : identifiants globaux, opérations des postes et suite
-- numérotée des changements (docs/spec-synchronisation.md, sections 3, 4 et 6).

-- Identifiant global (UUID version 7) des lignes synchronisées. Rempli par le
-- stockage après chaque ouverture et après chaque opération appliquée ; une
-- ligne créée par un poste reçoit l'identifiant choisi par ce poste. Plusieurs
-- NULL restent permis par l'index unique, le temps du remplissage.
ALTER TABLE utilisateur ADD COLUMN uid TEXT;
ALTER TABLE tiers ADD COLUMN uid TEXT;
ALTER TABLE produit ADD COLUMN uid TEXT;
ALTER TABLE document ADD COLUMN uid TEXT;
ALTER TABLE reglement ADD COLUMN uid TEXT;
CREATE UNIQUE INDEX utilisateur_uid ON utilisateur (uid);
CREATE UNIQUE INDEX tiers_uid ON tiers (uid);
CREATE UNIQUE INDEX produit_uid ON produit (uid);
CREATE UNIQUE INDEX document_uid ON document (uid);
CREATE UNIQUE INDEX reglement_uid ON reglement (uid);

-- Poste dont l'opération est en cours d'application, pour attribuer chaque
-- changement ; NULL hors de l'application d'une opération (changement fait sur
-- la machine même).
CREATE TABLE synchro_contexte (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    poste TEXT
) STRICT;
INSERT INTO synchro_contexte (id, poste) VALUES (1, NULL);

-- Suite numérotée des changements : une ligne par écriture d'une ligne
-- synchronisée. Le contenu n'y est pas copié : il est lu dans la ligne même au
-- moment où un poste demande les changements ; une ligne disparue est une
-- suppression.
CREATE TABLE changement (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    entite    TEXT NOT NULL,
    entite_id INTEGER NOT NULL,
    uid       TEXT,
    poste     TEXT
) STRICT;
CREATE INDEX changement_entite ON changement (entite, entite_id, seq);

-- Opérations reçues des postes, avec le résultat rendu : une opération renvoyée
-- (même identifiant, même contenu) reçoit le même résultat sans être appliquée
-- deux fois.
CREATE TABLE operation (
    id      TEXT PRIMARY KEY,
    poste   TEXT NOT NULL,
    compte  TEXT NOT NULL,
    contenu TEXT NOT NULL CHECK (json_valid(contenu)),
    issue   TEXT NOT NULL CHECK (json_valid(issue)),
    recue   TEXT NOT NULL
) STRICT;

-- Opérations ni appliquées ni rejetées : en attente de l'Admin (décisions 18
-- et 32) ou en conflit à trancher (décisions 17, 33 et 34).
CREATE TABLE operation_suspendue (
    operation_id TEXT PRIMARY KEY REFERENCES operation (id),
    motif        TEXT NOT NULL CHECK (motif IN ('attente', 'conflit')),
    cible        TEXT NOT NULL,
    ouverte      INTEGER NOT NULL DEFAULT 1 CHECK (ouverte IN (0, 1))
) STRICT;
CREATE INDEX operation_suspendue_cible ON operation_suspendue (cible, ouverte);

-- Suivi des écritures. Une mise à jour qui ne fait que poser l'identifiant
-- global n'est pas un changement.
CREATE TRIGGER utilisateur_suivi_creation AFTER INSERT ON utilisateur BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('utilisateur', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER utilisateur_suivi_modification AFTER UPDATE ON utilisateur
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('utilisateur', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER utilisateur_suivi_suppression AFTER DELETE ON utilisateur BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('utilisateur', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

CREATE TRIGGER tiers_suivi_creation AFTER INSERT ON tiers BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('tiers', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER tiers_suivi_modification AFTER UPDATE ON tiers
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('tiers', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER tiers_suivi_suppression AFTER DELETE ON tiers BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('tiers', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

CREATE TRIGGER produit_suivi_creation AFTER INSERT ON produit BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('produit', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER produit_suivi_modification AFTER UPDATE ON produit
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('produit', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER produit_suivi_suppression AFTER DELETE ON produit BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('produit', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

CREATE TRIGGER document_suivi_creation AFTER INSERT ON document BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER document_suivi_modification AFTER UPDATE ON document
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER document_suivi_suppression AFTER DELETE ON document BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Les lignes d'un document voyagent avec lui : leur écriture est un changement
-- du document. Lors d'une suppression en cascade, le document n'existe déjà
-- plus : l'identifiant reste vide, la suppression du document le porte.
CREATE TRIGGER ligne_document_suivi_creation AFTER INSERT ON ligne_document BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', NEW.document_id, (SELECT uid FROM document WHERE id = NEW.document_id),
            (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER ligne_document_suivi_modification AFTER UPDATE ON ligne_document BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', NEW.document_id, (SELECT uid FROM document WHERE id = NEW.document_id),
            (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER ligne_document_suivi_suppression AFTER DELETE ON ligne_document BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('document', OLD.document_id, (SELECT uid FROM document WHERE id = OLD.document_id),
            (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

CREATE TRIGGER reglement_suivi_creation AFTER INSERT ON reglement BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('reglement', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER reglement_suivi_modification AFTER UPDATE ON reglement
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('reglement', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER reglement_suivi_suppression AFTER DELETE ON reglement BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('reglement', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
