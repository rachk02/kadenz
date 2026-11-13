-- Phase 2, jalon F5 : réplique du serveur sur un poste relié
-- (docs/spec-synchronisation.md, section 6). Toutes les lignes que les postes
-- doivent recevoir entrent dans la suite des changements.

-- Instantanés de l'entreprise figés à la validation : identifiant global et suivi.
ALTER TABLE instantane_entreprise ADD COLUMN uid TEXT;
CREATE UNIQUE INDEX instantane_entreprise_uid ON instantane_entreprise (uid);

CREATE TRIGGER instantane_suivi_creation AFTER INSERT ON instantane_entreprise BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('instantane', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER instantane_suivi_modification AFTER UPDATE ON instantane_entreprise
WHEN NEW.uid IS OLD.uid BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('instantane', NEW.id, NEW.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER instantane_suivi_suppression AFTER DELETE ON instantane_entreprise BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('instantane', OLD.id, OLD.uid, (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Fiche de l'entreprise : une seule ligne, désignée par le code de l'entité.
CREATE TRIGGER entreprise_suivi_creation AFTER INSERT ON entreprise BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('entreprise', 1, 'entreprise', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER entreprise_suivi_modification AFTER UPDATE ON entreprise BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('entreprise', 1, 'entreprise', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Paramètres partagés par tous les postes, en un seul bloc. Seules les clés de
-- cette liste voyagent : les autres sont propres à chaque base (identité du
-- poste, licence V1, abonnement, port du service, dossiers d'export, suivi de
-- la réplique). La même liste est dans replique.rs (CLES_PARTAGEES), vérifiée
-- par un essai.
CREATE TRIGGER parametres_suivi_creation AFTER INSERT ON parametre_systeme
WHEN NEW.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER parametres_suivi_modification AFTER UPDATE ON parametre_systeme
WHEN NEW.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER parametres_suivi_suppression AFTER DELETE ON parametre_systeme
WHEN OLD.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Sur un poste : changements reçus qui n'ont pas encore pu être appliqués
-- (référence à une ligne pas encore reçue, valeur unique encore tenue par une
-- ligne qui va changer). Gardés avec le dernier rang reçu, réessayés à chaque
-- page ; la table est vide une fois le poste à jour.
CREATE TABLE synchro_attente (
    entite TEXT NOT NULL,
    uid    TEXT NOT NULL,
    seq    INTEGER NOT NULL,
    etat   TEXT NOT NULL CHECK (json_valid(etat)),
    PRIMARY KEY (entite, uid)
) STRICT;

-- Historique des lignes déjà présentes : sans lui, un serveur installé avant
-- cette migration n'enverrait jamais son entreprise, ses paramètres ni ses
-- instantanés. L'identifiant des instantanés est posé à l'ouverture suivante,
-- et lu au moment où un poste demande les changements.
INSERT INTO changement (entite, entite_id, uid, poste)
SELECT 'instantane', id, NULL, NULL FROM instantane_entreprise ORDER BY id;
INSERT INTO changement (entite, entite_id, uid, poste)
SELECT 'entreprise', 1, 'entreprise', NULL FROM entreprise;
INSERT INTO changement (entite, entite_id, uid, poste)
SELECT 'parametres', 1, 'parametres', NULL
WHERE EXISTS (SELECT 1 FROM parametre_systeme
              WHERE cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                            'lettres.orthographe', 'delais.validite_devis_jours',
                            'delais.paiement_jours', 'numerotation', 'documents.textes',
                            'documents.couleur', 'droits.matrice', 'relance.textes',
                            'impression.mise_en_page'));
