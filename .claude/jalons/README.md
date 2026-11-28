# Chapitres du journal (un fichier par jalon)

Le journal principal `.claude/journal.md` reste la seule source complète et chronologique :
chaque événement y est d'abord consigné. Les chapitres de ce dossier racontent un jalon
(lot, chantier, release) de bout en bout : objectif, décisions, travaux, reste à faire,
bilan. Ils ne recopient pas le journal, ils y renvoient par l'horodatage de l'entrée.

Règle globale de l'utilisateur (instructions globales, 2026-11-28), née dans le projet SMS
le 2026-11-17.

## Index

| Chapitre | Période | Statut |
|---|---|---|
| [Révocation des licences à distance](2026-11-revocation-licences.md) | 2026-11-27 | clos |
| [Retours de la 0.3.5 et coller depuis Word](2026-11-en-tete-word-et-abonnement.md) | depuis le 2026-11-27 | en cours |

Les jalons antérieurs au 2026-11-27 (phase 1, phase 2, portail, relais, édition monoposte,
manuel) restent à reconstituer à partir du journal et de git.

## Règles

1. **Un fichier par jalon**, nommé `AAAA-MM-sujet-court.md` (minuscules, sans accents ni
   espaces), ajouté à l'index ci-dessus avec sa période et son statut.
2. **Structure** : en-tête (objectif, périmètre, ouverture, clôture, statut `en cours` ou
   `clos`) ; `Décisions` (une ligne par décision, avec son horodatage) ; `Travaux` (une ligne
   par livraison, avec l'horodatage de l'entrée du journal et le commit) ; `Reste à faire et
   risques` ; `Bilan` à la clôture (livré, vérifié, reporté).
3. **Moments d'écriture** : à l'ouverture du jalon, à chaque décision ou livraison qui le
   concerne, à sa clôture. Pas à chaque entrée du journal.
4. **Le journal d'abord** : un événement est consigné dans `.claude/journal.md`, puis, s'il
   concerne un jalon, résumé dans son chapitre avec l'horodatage de l'entrée.
5. **Horodatage** : toujours pris sur l'horloge système (`date '+%Y-%m-%d %H:%M:%S'`).
6. **Mise à jour** : un chapitre peut être complété ou corrigé ; on ne supprime pas une ligne
   consignée, on ajoute une ligne qui la corrige.
7. **Style** : ni emoji, ni tiret cadratin, ni italique.
8. **Rien d'inventé** : chaque ligne s'appuie sur une entrée du journal, un commit ou une
   vérification faite.
