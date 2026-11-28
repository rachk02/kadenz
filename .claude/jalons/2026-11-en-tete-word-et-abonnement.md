# Retours de la 0.3.5 et coller depuis Word

- Objectif : corriger les retours de l'utilisateur sur la 0.3.5 et imprimer l'en-tête et le
  pied de page des documents comme dans Word.
- Périmètre : rubrique Abonnement, assistant de l'édition un poste, import Word, collage
  depuis Word, impression PDF, release 0.3.6.
- Ouverture : 2026-11-27 10:35:47. Statut : en cours.

## Décisions

- 2026-11-27 10:37:08 : proposer « Coller depuis Word » (image de la sélection copiée dans
  Word) plutôt qu'un import .docx fidèle, impossible à garantir ; accord de l'utilisateur
  (« oui lances toi »).
- 2026-11-27 11:10:16 : images collées bornées à 768 Ko chacune pour tenir dans une trame de
  8 Mo avec les deux logos.
- 2026-11-27 11:16:06 : release 0.3.6 demandée par l'utilisateur.

## Travaux

- 2026-11-27 10:35:47 : thème de l'assistant, champ du code replié, alerte d'activation,
  licences installées (7ea02c4).
- 2026-11-27 10:37:08 : import Word, `mc:Fallback` ignoré (faac12c).
- 2026-11-27 11:10:16 : coller l'en-tête et le pied de page depuis Word (e3edcb7).
- 2026-11-27 12:56:05 : release v0.3.6 publiée (0bfacb8) et servie par le portail.

## Reste à faire et risques

- 2026-11-27 17:53:41 : l'utilisateur signale que l'en-tête et le pied imprimés restent
  différents de Word, même après le collage de la 0.3.6.
- 2026-11-27 18:11:21 : diagnostic, les zones gardaient le texte d'un import antérieur à la
  correction et l'image collée n'était pas dans le PDF fourni ; une facture déjà validée garde
  son instantané.
- 2026-11-27 18:14:22 : proposition de l'utilisateur, en-tête sur le principe d'une signature
  de courriel (coller en gardant la mise en forme) ; trois voies présentées, choix attendu.

## Bilan

À remplir à la clôture.
