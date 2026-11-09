# Historique des versions

Format : une section par version, la plus récente en haut. Numérotation [SemVer](https://semver.org/lang/fr/) : `MAJEUR.MINEUR.CORRECTIF`, tag Git `vMAJEUR.MINEUR.CORRECTIF`.

## [Non publié]

### Ajouté
- Cahier des charges v2.0 (Rust, Slint, SQLite, mono-poste avec comptes et rôles).
- `kadenz-domaine` : statuts et transitions, calculs en entiers, numérotation, état de paiement, droits par rôle.
- `kadenz-stockage` : migrations embarquées, compteurs de numérotation transactionnels, audit en ajout seul, sauvegarde en ligne.
- Application Slint : fenêtre principale, navigation, écran À propos.
- Jalon J1 : type de document Avoir (préfixe A), date d'échéance et second logo dans le modèle de données ; montant en lettres en orthographe rectifiée ou traditionnelle ; saisie des pourcentages et des listes ; comptes utilisateurs (Argon2id, identifiant insensible à la casse, garde du dernier administrateur) ; fiche entreprise ; paramètres système ; journal d'audit ; installation initiale en une transaction ; écrans assistant de premier lancement, connexion et Mon compte (changement de mot de passe). Variable `KADENZ_BASE` pour travailler sur une autre base en développement.
- Jalon J2 : matrice des droits par rôle modifiable et contrôlée dans le stockage sur le compte relu en base ; menu construit selon les droits ; écrans Tiers et Produits (liste filtrable et triable, fiche, retrait qui supprime une fiche libre et désactive une fiche citée) ; écran Entreprise avec deux logos stockés dans la base ; écran Paramètres (taux, modes de paiement, délais facultatifs, orthographe du montant en lettres) ; écran Utilisateurs (comptes et droits par rôle) ; import CSV, Excel ou OpenDocument des tiers et des produits avec aperçu, doublons et erreurs par ligne, enregistrement des lignes valides en une transaction (crate `kadenz-import`).
- Jalon J3 : stockage des documents (brouillon aux totaux recalculés et au snapshot du tiers, validation numérotée en une transaction d'après le mois de la date, envoi, annulation qui garde le numéro, suppression des seuls brouillons tracée dans l'audit, champs administratifs après validation, état de paiement calculé, historique tiré de l'audit) ; saisie des quantités en millièmes ; écran Factures (registre filtrable et triable, fiche avec lignes modifiables, totaux et montant en lettres en direct, actions selon le statut et les droits, historique) ; identifiants des documents, tiers, produits et règlements jamais réutilisés après suppression ; échéance qui suit la date du document tant qu'elle n'a pas été modifiée à la main.
- Réglage de la numérotation (écran Paramètres) : préfixe par type, compteur remis à zéro chaque mois (`F2026-05-0001`, par défaut) ou chaque année (`F2026-0001`), nombre de chiffres du rang.
