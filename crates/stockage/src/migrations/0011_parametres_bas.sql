-- Phase 2, jalon F5 : les bas de document (impression.bas.<type>) sont des
-- paramètres partagés, oubliés par la migration 0010. Les déclencheurs du suivi
-- des paramètres sont recréés avec la liste complète, la même que
-- CLES_PARTAGEES dans replique.rs (vérifiée par un essai).

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
                 'impression.bas.avoir') BEGIN
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
                 'impression.bas.avoir') BEGIN
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
                 'impression.bas.avoir') BEGIN
    INSERT INTO changement (entite, entite_id, uid, poste)
    VALUES ('parametres', 1, 'parametres', (SELECT poste FROM synchro_contexte WHERE id = 1));
END;

-- Bas de document déjà enregistrés : les postes doivent les recevoir.
INSERT INTO changement (entite, entite_id, uid, poste)
SELECT 'parametres', 1, 'parametres', NULL
WHERE EXISTS (SELECT 1 FROM parametre_systeme WHERE cle LIKE 'impression.bas.%');
