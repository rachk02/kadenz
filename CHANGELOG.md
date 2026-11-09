# Historique des versions

Format : une section par version, la plus récente en haut. Numérotation [SemVer](https://semver.org/lang/fr/) : `MAJEUR.MINEUR.CORRECTIF`, tag Git `vMAJEUR.MINEUR.CORRECTIF`.

## [Non publié]

### Ajouté
- Cahier des charges v2.0 (Rust, Slint, SQLite, mono-poste avec comptes et rôles).
- `kadenz-domaine` : statuts et transitions, calculs en entiers, numérotation, état de paiement, droits par rôle.
- `kadenz-stockage` : migrations embarquées, compteurs de numérotation transactionnels, audit en ajout seul, sauvegarde en ligne.
- Application Slint : fenêtre principale, navigation, écran À propos.
- Jalon J1 : type de document Avoir (préfixe A), date d'échéance et second logo dans le modèle de données ; montant en lettres en orthographe rectifiée ou traditionnelle ; saisie des pourcentages et des listes ; comptes utilisateurs (Argon2id, identifiant insensible à la casse, garde du dernier administrateur) ; fiche entreprise ; paramètres système ; journal d'audit ; installation initiale en une transaction ; écrans assistant de premier lancement, connexion et Mon compte (changement de mot de passe). Variable `KADENZ_BASE` pour travailler sur une autre base en développement.
