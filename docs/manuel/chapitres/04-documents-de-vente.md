# 04 Documents de vente

Ce chapitre suit la vie d'une vente, du devis à l'avoir : on établit un devis, on le convertit en facture, on livre avec un bordereau, puis on corrige par un avoir. Les mêmes gestes servent à tous les documents, donc on les détaille une fois, sur le devis.

Les données sont celles du chapitre 03 : le client Boulangerie du Marché, les articles FAR-50 et SUC-25.

## 4.1 Les documents et leurs numéros

Le menu **Documents** regroupe six types de documents.

![Menu Documents](../captures/04-documents-vente/01-menu-documents.png)

- **Devis** : l'offre de prix, avant la vente.
- **Proforma** : une facture préalable, qui se traite comme un devis.
- **Bons de commande** : les commandes passées à vos fournisseurs (chapitre 05).
- **Livraisons** : les bordereaux de livraison, établis à partir d'une facture.
- **Factures** : la vente elle-même.
- **Avoirs** : la correction d'une facture.

Chaque document passe par des états :

- **Brouillon** : en préparation, modifiable, sans numéro. Il peut être supprimé.
- **Validé** : il reçoit son numéro définitif, et ses lignes et ses montants ne peuvent plus être modifiés.
- **Envoyé** : vous l'avez marqué comme transmis au client.
- **Converti** : un devis ou une proforma dont une facture a été validée.
- **Annulé** : le document reste dans le registre avec son numéro, en lecture seule.

Le numéro se forme d'une lettre, de l'année, du mois et d'un compteur : `D2026-11-0001` pour le premier devis de novembre 2026, `F` pour les factures, `L` pour les bordereaux, `A` pour les avoirs. Les préfixes se règlent dans les Paramètres, rubrique « Documents » (1 à 6 lettres, différentes d'un type à l'autre).

## 4.2 Établir un devis

### Le registre

![Registre des devis vide](../captures/04-documents-vente/02-devis-liste-vide.png)

Le menu **Documents**, rubrique **Devis**, ouvre le registre. Le bouton **Nouveau devis** crée un document. Le registre se retrouve à l'identique pour chaque type : une zone de recherche (numéro, client, libellé), des filtres par statut et par date, et les colonnes N°, Date, Client, Libellé, Montant TTC, Statut, Paiement, Mode et Échéance.

### Le formulaire

![Nouveau devis](../captures/04-documents-vente/03-nouveau-devis.png)

Le formulaire s'ouvre sur un bandeau « Statut : Brouillon » et la mention « Aucun numéro tant que le devis n'est pas validé. ». Le bouton **Retour au registre** referme le formulaire.

**Le client.** Tapez quelques lettres dans **Filtrer les clients** pour réduire la liste, puis choisissez le client dans la liste déroulante.

![Choix du client](../captures/04-documents-vente/04-choix-client.png)

Sous le client s'affichent son adresse, son IFU et son RCCM, tels qu'ils figureront sur le document. Le bouton **Nouveau client** crée un client sans quitter le devis ; sa fiche se complète ensuite dans les Tiers.

**La date, la validité et le libellé.** La **Date** est celle du jour, modifiable au format JJ/MM/AAAA. La **Validité** est facultative : c'est la date limite de l'offre. Le **Libellé** résume l'objet du document.

### Les lignes

Pour ajouter un article, choisissez-le dans la liste située sous le tableau, puis cliquez sur **Ajouter le produit**.

![Une ligne ajoutée](../captures/04-documents-vente/05-une-ligne.png)

Le produit remplit la référence, la désignation et le prix, qui restent modifiables sur la ligne. Vous pouvez aussi taper une référence puis appuyer sur Entrée : Kadenz reprend le produit correspondant. **Filtrer les produits** réduit la liste.

![Choix du produit](../captures/04-documents-vente/06-choix-produit.png)

Sur chaque ligne :

- la **Quantité** se modifie directement ;
- les flèches montent ou descendent la ligne ;
- le bouton `x` la supprime.

**Ligne libre** ajoute une ligne qui ne vient pas de la liste des produits : saisissez la désignation, le prix et la quantité. Cela convient à un travail ou à une prestation ponctuelle.

### Remise, TVA, mode de paiement et observations

Sous les lignes se trouvent la **Remise**, la **TVA**, le **Mode de paiement** et les **Observations**. Les valeurs proposées sont celles réglées à l'installation (chapitre 02).

![Choix de la TVA](../captures/04-documents-vente/07-choix-tva.png)

![Choix de la remise](../captures/04-documents-vente/08-choix-remise.png)

![Choix du mode de paiement](../captures/04-documents-vente/09-mode-paiement.png)

Les montants se calculent à droite, au fur et à mesure :

- **Montant HT** : la somme des lignes ;
- **Remise** : le pourcentage choisi, retiré du montant hors taxes ;
- **Base TVA** : le montant HT après remise ;
- **TVA** : le taux choisi, appliqué à la base ;
- **Montant TTC**, puis **En lettres**, le montant écrit en toutes lettres.

Dans l'exemple : 10 sacs de farine à 25 000 FCFA et 20 sacs de sucre à 18 000 FCFA font 610 000 FCFA hors taxes. Une remise de 5 % retire 30 500, la base de TVA est de 579 500, la TVA à 18 % est de 104 310, et le montant TTC est de 683 810 FCFA.

![Devis complet](../captures/04-documents-vente/10-devis-complet.png)

### Enregistrer, vérifier, valider

**Enregistrer le brouillon** garde le document sans lui donner de numéro. Le titre devient « Devis (brouillon) » et de nouveaux boutons apparaissent : **Aperçu**, **Exporter en Excel**, **Supprimer le brouillon** et **Dupliquer**.

![Brouillon enregistré](../captures/04-documents-vente/11-brouillon-enregistre.png)

**Aperçu** ouvre le document tel qu'il sera imprimé, avec le mot BROUILLON en filigrane tant qu'il n'est pas validé. Les boutons **Imprimer**, **Enregistrer en PDF** et **Fermer** sont en haut de la fenêtre. L'impression et les en-têtes sont détaillés au chapitre 08.

![Aperçu du brouillon](../captures/04-documents-vente/12-apercu-brouillon.png)

Quand tout est juste, cliquez sur **Valider**. Kadenz demande confirmation : « Valider ce devis ? Il reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés. »

![Confirmation de validation](../captures/04-documents-vente/13-valider-confirmation.png)

Cliquez sur **Confirmer la validation**. Le devis reçoit son numéro, ici D2026-11-0001, et son statut passe à **Validé**.

![Devis validé](../captures/04-documents-vente/14-devis-valide.png)

> **Attention** Après validation, plus rien ne se modifie sur les lignes et les montants. Relisez le brouillon avec Aperçu avant de valider.

### Que faire d'un document validé

Sur un devis validé, les boutons proposés sont :

- **Enregistrer le mode et les observations** : enregistre le mode de paiement et les observations, que l'on peut encore changer après validation ;
- **Marquer envoyé** : le statut passe à Envoyé, pour savoir où en est l'offre ;
- **Annuler le devis** : le devis reste dans le registre avec son numéro, en lecture seule ;
- **Aperçu**, **Exporter en PDF** et **Exporter en Excel** ;
- **Convertir en facture** (section 4.3) ;
- **Dupliquer** : crée une copie en brouillon, par exemple pour faire une nouvelle offre au même client ;
- **Corriger** : « Il est annulé, son numéro conservé, et une copie modifiable est créée en brouillon. »

![Devis marqué envoyé](../captures/04-documents-vente/15-marque-envoye.png)

Le bouton Corriger n'apparaît que sur un document validé qui n'a pas encore été converti, pour un utilisateur qui a le droit d'annuler un document validé (chapitre 10).

## 4.3 Convertir le devis en facture

Sur le devis validé ou envoyé, cliquez sur **Convertir en facture**. Kadenz crée une facture en brouillon avec les lignes, la remise, la TVA et le mode de paiement du devis, et affiche : « Facture créée en brouillon ; sa validation marquera le document d'origine comme converti. »

![Facture créée à partir du devis](../captures/04-documents-vente/16-conversion.png)

La facture a une **Échéance** facultative, la date limite de paiement ; saisissez-la au format JJ/MM/AAAA. Tout reste modifiable tant que la facture est en brouillon. La section **Documents liés**, en bas, renvoie au devis d'origine : un clic sur le bouton ouvre ce document.

Cliquez sur **Valider**, puis sur **Confirmer la validation**.

![Confirmation de validation de la facture](../captures/04-documents-vente/17-valider-facture.png)

La facture reçoit son numéro (F2026-11-0001), son statut est **Validé** et son paiement **Impayé**. Le devis passe à **Converti**.

![Facture validée](../captures/04-documents-vente/18-facture-validee.png)

La facture propose les mêmes boutons que le devis, plus :

- **Créer un bordereau** : le bordereau de livraison (section 4.4) ;
- **Créer un avoir** : la correction (section 4.5) ;
- la zone **Règlements**, pour saisir ce que le client paie. Elle est détaillée au chapitre 06.

## 4.4 Livrer avec un bordereau

Sur la facture validée, **Créer un bordereau** ouvre un bordereau de livraison en brouillon, repris des lignes de la facture : « Bordereau créé en brouillon avec les lignes de la facture : ajuster les quantités livrées puis valider. »

![Bordereau en brouillon](../captures/04-documents-vente/19-bordereau-brouillon.png)

Le bordereau ajoute des champs propres à la livraison : **Visa du client**, **Visa du fournisseur**, **Date de livraison** et **Date de réception**. Chaque ligne affiche la quantité **Commandée**, la quantité **Livrée** et le **Reste à livrer** avant ce bordereau.

Pour une livraison en plusieurs fois, baissez la quantité **Livrée** des lignes concernées. Dans l'exemple, on livre 6 sacs de farine sur 10, et les 20 sacs de sucre.

![Livraison partielle](../captures/04-documents-vente/20-bordereau-partiel.png)

Cliquez sur **Valider** puis **Confirmer la validation** : le bordereau reçoit son numéro (L2026-11-0001) et ses lignes ne se modifient plus. La livraison reste à confirmer : les boutons proposés sont **Enregistrer les visas et les observations** et **Confirmer la livraison**.

![Bordereau validé](../captures/04-documents-vente/22-bordereau-valide.png)

Cliquez sur **Confirmer la livraison**. Kadenz prévient que le statut passera en livraison partielle ou complète selon les quantités livrées, et que le bordereau ne pourra plus être annulé. Si le stock connu est insuffisant, il liste les produits qui passeront en stock négatif : la livraison reste possible.

![Confirmation de la livraison](../captures/04-documents-vente/23-livraison-confirmation.png)

Après confirmation, le statut du bordereau devient **Livraison partielle**, ou **Livraison complète** quand tout est livré.

![Livraison confirmée](../captures/04-documents-vente/24-livraison-confirmee.png)

Le stock est expliqué au chapitre 05.

## 4.5 Corriger avec un avoir

Une facture validée ne se modifie plus. Pour rectifier une erreur ou enregistrer un retour de marchandise, on établit un avoir. Sur la facture, cliquez sur **Créer un avoir** : Kadenz crée un avoir en brouillon avec toutes les lignes de la facture, et indique qu'il faut « retirer des lignes ou baisser les quantités pour un avoir partiel ».

![Avoir en brouillon](../captures/04-documents-vente/25-avoir-brouillon.png)

Dans l'exemple, le client retourne 2 sacs de sucre : on retire la ligne de farine avec le bouton `x`, on ramène la quantité de sucre à 2 et on change le libellé en « Retour de 2 sacs de sucre ». Les montants se recalculent.

![Avoir partiel](../captures/04-documents-vente/26-avoir-partiel.png)

Après **Valider** puis **Confirmer la validation**, l'avoir reçoit son numéro, A2026-11-0001, et reste lié à la facture d'origine.

![Avoir validé](../captures/04-documents-vente/27-avoir-valide.png)

## 4.6 Retrouver ses documents

### Les registres

Chaque type de document a son registre. Cliquez sur une ligne pour ouvrir le document.

![Registre des factures](../captures/04-documents-vente/28-registre-factures.png)

Le registre des devis montre le devis d'exemple au statut **Converti**, avec son échéance de validité.

![Registre des devis](../captures/04-documents-vente/29-registre-devis.png)

Le registre des avoirs :

![Registre des avoirs](../captures/04-documents-vente/30-registre-avoirs.png)

### Les filtres

Le filtre de statut propose **Tous les statuts**, **Brouillon**, **Validé**, **Envoyé** et **Annulé**. Dans le registre des factures, un second filtre porte sur le paiement (Impayé, Partiel, Payé : chapitre 06). Les deux zones de date, au format JJ/MM/AAAA, limitent la liste à une période, et la zone de recherche porte sur le numéro, le client et le libellé.

![Filtre de statut](../captures/04-documents-vente/31-filtre-statut.png)

Le bouton **Exporter en Excel** sort la liste affichée dans un classeur.

### Dupliquer et supprimer un brouillon

**Dupliquer** copie un document en brouillon, avec ses lignes. La section **Documents liés** garde l'origine de la copie.

![Copie en brouillon](../captures/04-documents-vente/32-dupliquer.png)

**Supprimer le brouillon** demande confirmation : « Il disparaît du registre ; la suppression reste tracée dans le journal d'audit. » Seul un brouillon peut être supprimé. Un document validé s'annule ou se corrige.

![Suppression d'un brouillon](../captures/04-documents-vente/33-supprimer-brouillon.png)

## 4.7 À retenir

- Un brouillon n'a pas de numéro et se modifie librement ; la validation donne le numéro et fige les lignes et les montants.
- Un devis ou une proforma se convertit en facture ; une facture donne un bordereau de livraison ou un avoir.
- On ne supprime pas un document validé : on l'annule, on le corrige ou on établit un avoir.
- Les documents liés se retrouvent en bas de chaque document, d'un clic.

Le chapitre suivant traite du stock, qui bouge avec les livraisons et les réceptions.
