-- Phase 2, jalon F7 : lignes de la copie d'un poste à revoir après une
-- restauration du serveur (nouvelle génération de sa base). Chaque ligne reçue
-- de nouveau en sort ; celles qui restent une fois le poste à jour n'existent
-- plus sur le serveur et sont retirées de la copie. Table propre au poste.

CREATE TABLE synchro_a_revoir (
    entite TEXT NOT NULL,
    uid    TEXT NOT NULL,
    PRIMARY KEY (entite, uid)
) STRICT;
