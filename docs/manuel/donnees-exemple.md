# Données d'exemple du manuel

Toutes les captures du manuel sont faites sur une installation vierge, avec les données fictives ci-dessous. Elles servent de fil rouge à tous les chapitres : on retrouve la même entreprise, le même administrateur et, plus tard, les mêmes clients et produits.

Règle de confidentialité : aucune capture ne doit montrer un nom d'utilisateur Windows réel, un chemin de dossier personnel, une adresse ou un numéro réels. Le dossier des sauvegardes d'exemple est `C:\Sauvegardes-Kadenz`.

## Entreprise

| Champ | Valeur |
|---|---|
| Nom de l'entreprise | Comptoir Exemple SARL |
| Adresse / boîte postale | 01 BP 1234 Ouagadougou 01 |
| Téléphone | +226 25 00 00 00 |
| Email | contact@exemple.bf |
| RCCM | BF-OUA-2024-B-0000 |
| IFU | 00000000A |

## Administrateur

| Champ | Valeur |
|---|---|
| Nom complet | Awa Exemple |
| Identifiant de connexion | admin |
| Mot de passe | Exemple-2026 |

## Sauvegardes

| Champ | Valeur |
|---|---|
| Dossier | C:\Sauvegardes-Kadenz |
| Heure quotidienne | 12:00 (valeur par défaut) |
| Mot de passe des sauvegardes | Sauvegarde-2026 |

## Taux et paiements

Valeurs par défaut de l'assistant, conservées : TVA 0 ; 18, remise 0 ; 5 ; 10 ; 15, retenue 0 ; 5 ; 10 ; 15, modes de paiement Espèce, Chèque, Virement bancaire, Carte bancaire, Moov money, Orange money, montant en lettres en orthographe traditionnelle.

## Mode opératoire des captures

- Lancer : `powershell -File docs\manuel\outils\lancer.ps1 -Edition monoposte -Nom vierge -Remise` (données isolées dans `%TEMP%\kadenz-manuel\vierge`).
- Piloter et capturer : `powershell -File docs\manuel\outils\pilote.ps1 -Sortie docs\manuel\captures\<chapitre> -Actions "..."` (voir l'en-tête du script pour les actions).
- Fenêtre : 1280 x 800, thème clair, français, ABC Diatype.
- Les captures sont celles de l'écran : ne pas toucher à la souris ni au clavier pendant l'exécution. Le pilote refuse d'agir si Kadenz n'est pas la fenêtre active.
