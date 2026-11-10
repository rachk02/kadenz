-- Bordereau de livraison (jalon J6, section 6.5) : zones de visa client et
-- fournisseur, dates de livraison et de réception, conservées en texte libre.
-- Vides pour les autres types de documents. Le lien d'un bordereau vers sa
-- facture passe par document_source_id, sans nature_source (la contrainte de la
-- migration 0004 ne connaît pas « livraison ») : voir documents::nature_lue.
ALTER TABLE document ADD COLUMN visa_client TEXT NOT NULL DEFAULT '';
ALTER TABLE document ADD COLUMN visa_fournisseur TEXT NOT NULL DEFAULT '';
ALTER TABLE document ADD COLUMN date_livraison_texte TEXT NOT NULL DEFAULT '';
ALTER TABLE document ADD COLUMN date_reception_texte TEXT NOT NULL DEFAULT '';
