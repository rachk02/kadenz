# Révocation des licences à distance

- Objectif : révoquer une licence à distance, voir chaque appareil relié à une licence et le
  déconnecter un par un (demande de l'utilisateur, « comme Kaspersky »).
- Périmètre : bulletin signé du portail, pages du portail, serveur du client, application,
  confidentialité, release.
- Spécification : `docs/spec-revocation-licences.md` (décision 80).
- Ouverture : 2026-11-27 05:43:24. Clôture : 2026-11-27 08:57:24. Statut : clos.

## Décisions

- 2026-11-27 05:51:49 : 30 jours sans contact avec le portail, puis lecture seule (licence
  installée seulement, essai exempté) ; appareil déconnecté bloqué par l'identifiant de
  l'ordinateur jusqu'à réautorisation.
- 2026-11-27 06:06:55 : la migration 14 du portail ne part en production qu'avec les pages et
  les postes clients prêts ; une seule mise à jour du Dell.
- 2026-11-27 07:57:13 : accord de l'utilisateur pour le lot R6 (confidentialité, LISEZMOI,
  notes 0.3.5).

## Travaux

- 2026-11-27 05:53:34 : lot R1, bulletin signé `KADENZ-B-` (6b8cc6b).
- 2026-11-27 06:00:53 : lot R2, schéma 14 du portail, route `/api/licence/etat` (4f38999,
  essais remis par d361f1d).
- 2026-11-27 06:10:01 : lot R3, rubriques Appareils, Ordinateurs bloqués et Révocation
  (e5adbd0).
- 2026-11-27 06:44:28 : lot R4, serveur du client, migration 0020, 30 jours sans contact
  (5c0b5d7).
- 2026-11-27 07:51:42 : lot R5, échange de l'application avec le portail toutes les 6 heures
  (ec1882b).
- 2026-11-27 08:02:17 : portail 4a513229cc66 déployé sur le Dell.
- 2026-11-27 08:54:07 : release v0.3.5 publiée (4a51322).
- 2026-11-27 08:57:24 : paquets 0.3.5 dans `/srv/kadenz/depot`, servis par le portail.

## Reste à faire et risques

- « Changer d'ordinateur » reporté jusqu'à la demande d'un revendeur.

## Bilan

Livré et en production : révocation, déconnexion poste par poste, ordinateurs bloqués,
lecture seule après 30 jours sans confirmation. Vérifié par les essais automatiques, la CI et
les contrôles du portail en production.
