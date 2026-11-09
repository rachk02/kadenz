# Journal du projet Kadenz

[2026-11-09 09:49:49] Instruction permanente - Règles de travail de l'utilisateur reçues de la session setic-f0 : réponses en français, journal horodaté, aucune attribution d'outil dans les commits et PR, aucun emoji ni tiret cadratin ni italique, préférence d'angles droits pour le web, dialogue entre sessions, livre blanc s'il existe, liste SEO pour tout site public, ne rien inventer.
[2026-11-09 09:53:18] Remarque - L'utilisateur signale que le cahier des charges (docs_of_interest/kadenz-cahier-des-charges.md) ne correspond plus au projet voulu ; analyse du cahier et des captures Excel avant toute mise en place.
[2026-11-09 09:54:23] Instruction permanente - Changement de cible : Kadenz passe en Rust, application desktop mono-poste avec SQLite (remplace .NET 8 / WPF / EF Core du cahier des charges).
[2026-11-09 09:59:48] Instruction permanente - Comptes utilisateurs et rôles conservés en mono-poste ; le verrouillage optimiste multi-poste (RowVersion) est retiré.
[2026-11-09 09:59:48] Suggestion - Choix délégué par l'utilisateur ("native, ultra performante, à vendre aux entreprises") : interface Slint 1.18 (licence Royalty-free 2.0 vérifiée, gratuite pour le desktop sous condition d'afficher AboutSlint dans un écran À propos).
[2026-11-09 09:59:48] Suggestion - Choix délégué ("meilleure solution, la plus modulable") : module Paiements commun à la Facture (créance client) et au Bon de commande (dette fournisseur), état calculé à partir des règlements ; modularité = crates séparées + modules activables par installation.
[2026-11-09 10:10:13] Instruction permanente - Mettre en place le dépôt GitHub : l'ancien dépôt privé rachk02/kadenz (.NET puis Flutter) est renommé en archive, un nouveau dépôt privé rachk02/kadenz accueille le projet Rust ; les captures de docs_of_interest/ sont versionnées.
[2026-11-09 10:10:13] Suivi - Cahier des charges v2.0 (docs/cahier-des-charges.md) et squelette Rust : crates kadenz-domaine, kadenz-stockage, kadenz (Slint) ; 24 tests unitaires au vert.
[2026-11-09 10:10:13] Remarque - L'utilisateur veut discuter ensuite de toutes les fonctionnalités et de tout ce qui doit être modulable.
