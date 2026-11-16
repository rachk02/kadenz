-- Phase 2, export comptable SYSCOHADA (jalon C1, docs/spec-export-syscohada.md).

-- Nature comptable d'un produit (4.4), figée sur la ligne à la validation : un
-- mois clos ne change pas si la fiche change ensuite. Une ligne libre, sans
-- produit, compte comme marchandise.
ALTER TABLE produit ADD COLUMN nature_comptable TEXT NOT NULL DEFAULT 'marchandise'
    CHECK (nature_comptable IN ('marchandise', 'service'));
ALTER TABLE ligne_document ADD COLUMN nature_snapshot TEXT NOT NULL DEFAULT 'marchandise'
    CHECK (nature_snapshot IN ('marchandise', 'service'));

-- Comptes (comptabilite.comptes) et mois clos (comptabilite.clotures) sont des
-- paramètres partagés : les déclencheurs du suivi sont recréés avec la liste
-- complète, la même que CLES_PARTAGEES dans replique.rs (vérifiée par un essai).
DROP TRIGGER parametres_suivi_creation;
DROP TRIGGER parametres_suivi_modification;
DROP TRIGGER parametres_suivi_suppression;

CREATE TRIGGER parametres_suivi_creation AFTER INSERT ON parametre_systeme
WHEN NEW.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page', 'impression.bas.devis',
                 'impression.bas.proforma', 'impression.bas.bon_commande',
                 'impression.bas.livraison', 'impression.bas.facture',
                 'impression.bas.avoir', 'comptabilite.comptes',
                 'comptabilite.clotures') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER parametres_suivi_modification AFTER UPDATE ON parametre_systeme
WHEN NEW.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page', 'impression.bas.devis',
                 'impression.bas.proforma', 'impression.bas.bon_commande',
                 'impression.bas.livraison', 'impression.bas.facture',
                 'impression.bas.avoir', 'comptabilite.comptes',
                 'comptabilite.clotures') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;
CREATE TRIGGER parametres_suivi_suppression AFTER DELETE ON parametre_systeme
WHEN OLD.cle IN ('taux.tva', 'taux.remise', 'taux.retenue', 'paiement.modes',
                 'lettres.orthographe', 'delais.validite_devis_jours',
                 'delais.paiement_jours', 'numerotation', 'documents.textes',
                 'documents.couleur', 'droits.matrice', 'relance.textes',
                 'impression.mise_en_page', 'impression.bas.devis',
                 'impression.bas.proforma', 'impression.bas.bon_commande',
                 'impression.bas.livraison', 'impression.bas.facture',
                 'impression.bas.avoir', 'comptabilite.comptes',
                 'comptabilite.clotures') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Droit nouveau, ajouté à l'Admin et au Comptable d'une matrice déjà
-- enregistrée (sans elle, la matrice par défaut du code le donne déjà).
UPDATE parametre_systeme
SET valeur = json_insert(valeur, '$.admin[#]', 'exporter_comptabilite')
WHERE cle = 'droits.matrice' AND json_valid(valeur) AND json_type(valeur, '$.admin') = 'array';
UPDATE parametre_systeme
SET valeur = json_insert(valeur, '$.comptable[#]', 'exporter_comptabilite')
WHERE cle = 'droits.matrice' AND json_valid(valeur) AND json_type(valeur, '$.comptable') = 'array';
