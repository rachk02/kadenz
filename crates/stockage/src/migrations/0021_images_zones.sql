-- Images de l'en-tête et du pied de page collées depuis Word (variables
-- {image_entete} et {image_pied}) : gardées comme les logos, dans la fiche de
-- l'entreprise et dans l'instantané pris à la validation d'un document.
ALTER TABLE entreprise ADD COLUMN image_entete BLOB;
ALTER TABLE entreprise ADD COLUMN image_pied BLOB;
ALTER TABLE instantane_entreprise ADD COLUMN image_entete BLOB;
ALTER TABLE instantane_entreprise ADD COLUMN image_pied BLOB;
