-- Phase 2, jalon F6, décision 52 : comptes d'un poste relié dont la connexion a
-- été refusée par le serveur (compte désactivé, mot de passe changé). Leurs
-- écritures de la file d'envoi partent sous la session d'un autre compte, et
-- le serveur les met en attente pour l'Admin. Table propre au poste.

CREATE TABLE synchro_compte_refuse (
    uid TEXT PRIMARY KEY
) STRICT;
