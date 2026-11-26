# 15 Dépannage et annexes

Ce chapitre rassemble les situations qui inquiètent le plus souvent, avec ce qu'il faut regarder et faire, puis des annexes : conventions de saisie, emplacement des fichiers, raccourcis clavier et petit lexique. Chaque réponse renvoie au chapitre qui la détaille.

## 15.1 Avant de demander de l'aide

Notez toujours :

- la **version** de Kadenz : **Paramètres**, rubrique **À propos** (chapitre 13) ;
- ce que vous faisiez et le **message exact** affiché à l'écran ;
- l'édition : un poste (serveur intégré) ou multiposte ;
- le contenu du fichier `erreurs.log` ou `plantage.log` s'il existe (section 15.4).

## 15.2 Les messages que l'on rencontre le plus

### Je ne peux plus rien saisir ni valider

- Le bandeau ou la mention dit **lecture seule** : l'essai gratuit ou l'abonnement est terminé. Consultation, recherche, impression et exports restent possibles. Activez un code d'abonnement (chapitre 12, sections 12.3 et 12.4).
- Le document est déjà **validé** : après validation, seuls le mode de paiement et les observations changent. On corrige par un avoir (chapitre 04).
- Le mois est **clos** : aucune facture ni aucun avoir daté de ce mois ne peut être validé ou annulé, et aucun règlement de ce mois ne peut être ajouté ou supprimé. Passez par un avoir daté d'un mois ouvert (chapitre 09, section 9.3).

### Le bouton « Valider » est inactif

Hors connexion, la validation attend le serveur, qui attribue le numéro (chapitre 13, section 13.5). Rétablissez la liaison, puis validez.

### La mention « Hors connexion » ou « Serveur non reconnu » s'affiche

- **Hors connexion : serveur injoignable** : le serveur est éteint ou le réseau est coupé. Vérifiez que la machine du serveur est allumée et que le service est **Démarré** (chapitre 13, section 13.4). Votre travail attend dans la file d'envoi et partira au retour du serveur.
- **Serveur non reconnu** : le certificat du serveur a changé. La connexion est bloquée volontairement. Demandez à l'Admin de comparer l'empreinte du certificat (chapitre 13, section 13.4) avant d'inscrire de nouveau le poste.
- **Poste retiré du serveur** : l'Admin a retiré ce poste. Il faut un nouveau code d'inscription (chapitre 13, section 13.2).
- **Mise à jour de ce poste requise** ou **Mise à jour du serveur attendue** : les versions ne correspondent pas. Mettez le serveur à jour d'abord, puis les postes (chapitre 12, section 12.6).

### Le code d'abonnement est refusé

Un message rouge dit « Code d'activation inconnu : vérifiez la saisie. » Recopiez le code avec soin : les lettres O et I et les chiffres 0 et 1 n'existent pas dans un code. L'activation demande Internet (chapitre 12, section 12.3).

### Un code d'inscription de poste ne marche pas

Il sert une fois, pour un poste, pendant 24 heures, et un nouveau code pour le même nom de poste annule l'ancien. Demandez à l'Admin d'en créer un nouveau (chapitre 13, section 13.4). Avec un code court, vérifiez l'adresse du serveur.

### Le stock est négatif

Kadenz laisse livrer sans stock et le signale. Corrigez par la réception d'un bon de commande, un ajustement ou un inventaire (chapitre 05, sections 5.5 à 5.7). Enregistrez le stock de départ avant les premières livraisons pour obtenir des coûts justes.

### Un import affiche des erreurs

Seules les lignes valides sont importées. Corrigez les lignes en erreur dans votre fichier et importez-le de nouveau : les lignes déjà importées sortent en doublons (chapitre 07, section 7.2).

### Une facture n'apparaît pas dans les relances

Elle n'a pas d'échéance, ou son échéance n'est pas dépassée, ou elle est soldée. Renseignez l'échéance dans la facture (chapitre 06, section 6.6).

### Le journal comptable est vide

Dans la version 0.3.1, à l'ouverture de la page **Comptabilité**, rechoisissez le mois dans la liste ; les versions suivantes affichent le journal directement (chapitre 09, section 9.2).

### Un export ne contient pas les bons comptes

Les comptes se règlent dans **Paramètres**, rubrique **Comptabilité** ; un mode de paiement sans compte va au compte d'attente. Un mois clos garde les comptes en vigueur au moment de sa clôture (chapitre 09, sections 9.1 et 9.3).

## 15.3 L'affichage est incorrect

### Des champs de saisie sont invisibles ou mal affichés

Depuis la version 0.3.2, Kadenz dessine sa fenêtre par le processeur (**rendu logiciel**), sans passer par le pilote graphique : les champs de saisie invisibles et l'affichage incomplet dus aux pilotes ne devraient plus se produire. Pour essayer le rendu accéléré par la carte graphique, plus rapide sur un poste récent :
- lancer Kadenz avec l'option `--rendu-accelere` ;
- ou créer un fichier vide nommé `rendu-accelere` dans le dossier des données (section 15.4) ;
- ou définir la variable d'environnement `KADENZ_RENDU` à la valeur `accelere`.
Si Kadenz s'est fermé brutalement au démarrage (arrêt du pilote graphique, par exemple), il revient seul au rendu logiciel au lancement suivant : le fichier `rendu-accelere` est supprimé et le fichier `rendu-logiciel` est créé. Pour forcer le rendu logiciel, créez le fichier `rendu-logiciel` ou lancez Kadenz avec `--rendu-logiciel` ; il l'emporte sur le rendu accéléré.

## 15.4 Où sont mes fichiers ?

| Quoi | Où |
| --- | --- |
| Données de Kadenz (édition un poste) | le dossier `Kadenz` de votre profil Windows (`%APPDATA%\Kadenz`) |
| Journal des erreurs techniques | `erreurs.log`, à côté de la base ; il repart de zéro au-delà d'un mégaoctet |
| Journal de plantage | `journaux\plantage.log` du dossier des données ; il ne contient aucune donnée de l'entreprise |
| Sauvegardes | le dossier choisi dans **Paramètres**, **Sauvegardes**, fichiers `kadenz-<type>-AAAAMMJJ-HHMMSS.kzs` (chapitre 11) |
| Modèles d'import | le dossier `modeles` de l'installation (chapitre 07) |
| PDF exportés | le **Dossier d'export** de **Paramètres**, **Textes imprimés et exports** (chapitre 08) |

Le journal de plantage peut être envoyé au support : il contient la cause, le lieu et la pile de l'erreur, jamais vos factures.

## 15.5 Si Kadenz ne démarre plus

Kadenz affiche l'écran **Démarrage impossible** quand la base ou sa clé est illisible. Restaurez une sauvegarde avec son mot de passe (chapitre 11, section 11.6). Rien n'est jamais effacé : les anciens fichiers sont mis de côté.

## 15.6 Annexe A : conventions de saisie

- **Montants** : en FCFA, en nombres entiers, sans espace ni symbole ni décimale.
- **Dates** : au format JJ/MM/AAAA.
- **Taux** (TVA, remise, retenue) : en pourcentages séparés par des points-virgules dans les réglages (chapitre 08, section 8.2).
- **Codes d'activation** : 25 caractères en 5 groupes de 5, sans O, I, 0 ni 1 (chapitre 12).
- **Codes d'inscription de poste** : un code complet à coller, ou un code court `XXXX-XXXX-XXXX` avec l'adresse du serveur (chapitre 13).
- **Mots de passe** : 8 caractères au moins.

## 15.7 Annexe B : raccourcis clavier

Dans les listes et tableaux, après avoir cliqué sur une ligne :

| Touche | Effet |
| --- | --- |
| Flèche du haut | ligne précédente |
| Flèche du bas | ligne suivante |
| Début | première ligne |
| Fin | dernière ligne |
| Page précédente | une page vers le haut |
| Page suivante | une page vers le bas |

## 15.8 Annexe C : lexique

| Terme | Sens |
| --- | --- |
| **Abonnement** | droit d'utiliser Kadenz, pour l'entreprise et tous ses postes, activé par un code |
| **Avoir** | document qui annule tout ou partie d'une facture validée |
| **Base** | le fichier qui contient toutes vos données |
| **Bon de commande** | document d'achat adressé à un fournisseur |
| **Bordereau de livraison** | document qui accompagne la sortie de marchandise ; sa confirmation sort le stock |
| **Brouillon** | document pas encore validé, modifiable |
| **Clôture** | action qui fige un mois comptable, sans retour possible |
| **Coût moyen** | prix moyen d'entrée d'un produit, qui valorise le stock |
| **Échéance** | date limite de paiement d'une facture |
| **Hors connexion** | état d'un poste qui ne joint plus son serveur |
| **Lecture seule** | état de Kadenz après la fin de l'abonnement : on consulte, on ne saisit plus |
| **Poste** | ordinateur sur lequel on travaille avec Kadenz |
| **Relais** | passerelle de l'éditeur qui relie un poste hors du bureau au serveur |
| **Serveur** | programme qui garde la base et attribue les numéros |
| **SYSCOHADA** | plan comptable des pays de l'espace OHADA |
| **Validation** | passage d'un brouillon à un document définitif, qui reçoit son numéro |

Les chapitres précédents détaillent chaque notion.
