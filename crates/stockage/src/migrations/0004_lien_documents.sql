-- Nature du lien entre un document et sa source (jalon J5, section 7.7) :
-- conversion (facture issue d'un devis ou d'une proforma), correction (copie
-- d'un devis ou d'une proforma dont l'original est annulé), copie (simple
-- duplication), avoir (avoir rattaché à sa facture). Vide sans source.
ALTER TABLE document ADD COLUMN nature_source TEXT
    CHECK (nature_source IS NULL OR nature_source IN ('conversion', 'correction', 'copie', 'avoir'));

CREATE INDEX document_par_source ON document (document_source_id);
