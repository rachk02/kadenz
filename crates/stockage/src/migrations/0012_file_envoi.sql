-- Phase 2, jalon F6 : file d'envoi d'un poste relié. Les écritures faites
-- hors connexion sont appliquées à titre provisoire dans la copie et gardées
-- ici, dans l'ordre, avec les lignes qu'elles touchent ; elles partent au
-- retour du serveur. Une opération suspendue sur le serveur (conflit ou
-- attente) reste notée, pour apprendre la décision en la renvoyant.
-- Tables propres au poste : jamais transmises.

CREATE TABLE synchro_file (
    rang      INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL CHECK (json_valid(operation)),
    lignes    TEXT NOT NULL CHECK (json_valid(lignes)),
    etat      TEXT NOT NULL DEFAULT 'envoi' CHECK (etat IN ('envoi', 'suspendue'))
) STRICT;

-- Brouillons dont la validation a été demandée hors connexion (écrans,
-- section 4) : proposés à la validation au retour du serveur.
CREATE TABLE synchro_a_valider (
    uid TEXT PRIMARY KEY
) STRICT;
