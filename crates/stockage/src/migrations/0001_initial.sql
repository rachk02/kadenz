-- Schéma initial de Kadenz (cahier des charges, section 11).
-- Modifiable tant qu'aucune version n'est publiée ; figé à partir de la v0.1.0,
-- après quoi toute évolution passe par une nouvelle migration.
-- Montants en FCFA entiers, pourcentages en centièmes de point, quantités en millièmes,
-- dates en TEXT ISO 8601, valeurs structurées en JSON dans des colonnes TEXT.
-- AUTOINCREMENT sur les tables dont les lignes peuvent être supprimées et sont citées
-- par le journal d'audit : un identifiant n'y est jamais réutilisé, l'historique d'une
-- fiche ou d'un document supprimé ne se mêle pas à celui d'un nouveau.

CREATE TABLE entreprise (
    id                        INTEGER PRIMARY KEY CHECK (id = 1),
    nom                       TEXT NOT NULL,
    adresse                   TEXT NOT NULL DEFAULT '',
    telephone                 TEXT NOT NULL DEFAULT '',
    email                     TEXT NOT NULL DEFAULT '',
    site_web                  TEXT NOT NULL DEFAULT '',
    localisation              TEXT NOT NULL DEFAULT '',
    regime_imposition         TEXT NOT NULL DEFAULT '',
    rccm                      TEXT NOT NULL DEFAULT '',
    ifu                       TEXT NOT NULL DEFAULT '',
    banque1                   TEXT NOT NULL DEFAULT '',
    banque2                   TEXT NOT NULL DEFAULT '',
    -- Logos stockés dans la base (PNG ou JPEG) : ils suivent sauvegardes et restaurations.
    logo                      BLOB,
    logo2                     BLOB,
    responsable_facturation   TEXT NOT NULL DEFAULT '',
    responsable_comptabilite  TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE utilisateur (
    id                 INTEGER PRIMARY KEY,
    nom_complet        TEXT NOT NULL,
    identifiant        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    mot_de_passe_hache TEXT NOT NULL,
    role               TEXT NOT NULL CHECK (role IN ('admin', 'comptable', 'commercial', 'consultation')),
    actif              INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    date_creation      TEXT NOT NULL
) STRICT;

CREATE TABLE tiers (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    nom               TEXT NOT NULL,
    adresse           TEXT NOT NULL DEFAULT '',
    telephone         TEXT NOT NULL DEFAULT '',
    localisation      TEXT NOT NULL DEFAULT '',
    ifu               TEXT NOT NULL DEFAULT '',
    rccm              TEXT NOT NULL DEFAULT '',
    regime_imposition TEXT NOT NULL DEFAULT '',
    type              TEXT NOT NULL CHECK (type IN ('client', 'fournisseur', 'les_deux')),
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1))
) STRICT;

CREATE TABLE produit (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    designation TEXT NOT NULL,
    reference   TEXT NOT NULL UNIQUE COLLATE NOCASE,
    prix        INTEGER NOT NULL CHECK (prix >= 0),
    sens        TEXT NOT NULL CHECK (sens IN ('vente', 'achat', 'les_deux')),
    actif       INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1))
) STRICT;

CREATE TABLE document (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    type                      TEXT NOT NULL CHECK (type IN ('devis', 'proforma', 'bon_commande', 'livraison', 'facture', 'avoir')),
    numero                    TEXT,
    date                      TEXT NOT NULL,
    tiers_id                  INTEGER REFERENCES tiers (id),
    tiers_nom_snapshot        TEXT NOT NULL DEFAULT '',
    tiers_adresse_snapshot    TEXT NOT NULL DEFAULT '',
    tiers_ifu_snapshot        TEXT NOT NULL DEFAULT '',
    tiers_rccm_snapshot       TEXT NOT NULL DEFAULT '',
    tiers_regime_snapshot     TEXT NOT NULL DEFAULT '',
    statut                    TEXT NOT NULL DEFAULT 'brouillon' CHECK (statut IN (
                                  'brouillon', 'valide', 'envoye', 'converti',
                                  'reception_partielle', 'reception_complete',
                                  'livraison_partielle', 'livraison_complete', 'annule')),
    libelle                   TEXT NOT NULL DEFAULT '',
    montant_ht                INTEGER NOT NULL DEFAULT 0,
    montant_remise            INTEGER NOT NULL DEFAULT 0,
    montant_base_tva          INTEGER NOT NULL DEFAULT 0,
    montant_tva               INTEGER NOT NULL DEFAULT 0,
    montant_retenue           INTEGER NOT NULL DEFAULT 0,
    montant_ttc               INTEGER NOT NULL DEFAULT 0,
    pourcentage_remise        INTEGER NOT NULL DEFAULT 0,
    pourcentage_tva           INTEGER NOT NULL DEFAULT 0,
    pourcentage_retenue       INTEGER NOT NULL DEFAULT 0,
    mode_paiement             TEXT NOT NULL DEFAULT '',
    observations              TEXT NOT NULL DEFAULT '',
    chemin_pdf                TEXT NOT NULL DEFAULT '',
    document_source_id        INTEGER REFERENCES document (id),
    date_echeance             TEXT,
    date_creation             TEXT NOT NULL,
    date_validation           TEXT,
    date_envoi                TEXT,
    utilisateur_creation_id   INTEGER REFERENCES utilisateur (id),
    utilisateur_validation_id INTEGER REFERENCES utilisateur (id),
    UNIQUE (type, numero),
    -- Un numéro existe dès la validation, jamais en brouillon (section 7.2).
    -- Un document annulé garde son numéro s'il avait été validé, sinon il n'en a pas.
    CHECK (statut <> 'brouillon' OR numero IS NULL),
    CHECK (statut IN ('brouillon', 'annule') OR numero IS NOT NULL)
) STRICT;

CREATE INDEX document_par_type_date ON document (type, date);
CREATE INDEX document_par_tiers ON document (tiers_id);

CREATE TABLE ligne_document (
    id                    INTEGER PRIMARY KEY,
    document_id           INTEGER NOT NULL REFERENCES document (id) ON DELETE CASCADE,
    produit_id            INTEGER REFERENCES produit (id),
    reference_snapshot    TEXT NOT NULL DEFAULT '',
    designation_snapshot  TEXT NOT NULL DEFAULT '',
    prix_snapshot         INTEGER NOT NULL DEFAULT 0,
    quantite              INTEGER NOT NULL DEFAULT 0,
    quantite_livree       INTEGER NOT NULL DEFAULT 0,
    total                 INTEGER NOT NULL DEFAULT 0,
    observations          TEXT NOT NULL DEFAULT '',
    ordre                 INTEGER NOT NULL
) STRICT;

CREATE INDEX ligne_par_document ON ligne_document (document_id, ordre);

CREATE TABLE reglement (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id    INTEGER NOT NULL REFERENCES document (id),
    date           TEXT NOT NULL,
    montant        INTEGER NOT NULL CHECK (montant > 0),
    mode_paiement  TEXT NOT NULL DEFAULT '',
    reference      TEXT NOT NULL DEFAULT '',
    utilisateur_id INTEGER REFERENCES utilisateur (id)
) STRICT;

CREATE INDEX reglement_par_document ON reglement (document_id);

-- Seuls la Facture et le Bon de commande reçoivent des règlements (section 8).
CREATE TRIGGER reglement_type_document
BEFORE INSERT ON reglement
WHEN (SELECT type FROM document WHERE id = NEW.document_id) NOT IN ('facture', 'bon_commande')
BEGIN
    SELECT RAISE(ABORT, 'un règlement ne peut viser qu''une facture ou un bon de commande');
END;

CREATE TABLE compteur_numerotation (
    type_document  TEXT NOT NULL,
    annee          INTEGER NOT NULL,
    mois           INTEGER NOT NULL CHECK (mois BETWEEN 1 AND 12),
    dernier_numero INTEGER NOT NULL,
    PRIMARY KEY (type_document, annee, mois)
) STRICT;

CREATE TABLE parametre_systeme (
    cle    TEXT PRIMARY KEY,
    valeur TEXT NOT NULL CHECK (json_valid(valeur))
) STRICT;

CREATE TABLE audit_log (
    id               INTEGER PRIMARY KEY,
    entite           TEXT NOT NULL,
    entite_id        INTEGER NOT NULL,
    action           TEXT NOT NULL,
    ancienne_valeur  TEXT CHECK (ancienne_valeur IS NULL OR json_valid(ancienne_valeur)),
    nouvelle_valeur  TEXT CHECK (nouvelle_valeur IS NULL OR json_valid(nouvelle_valeur)),
    utilisateur_id   INTEGER REFERENCES utilisateur (id),
    date_action      TEXT NOT NULL
) STRICT;

-- Journal d'audit en ajout seul (section 7.4).
CREATE TRIGGER audit_log_sans_modification
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'le journal d''audit est en ajout seul');
END;

CREATE TRIGGER audit_log_sans_suppression
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'le journal d''audit est en ajout seul');
END;
