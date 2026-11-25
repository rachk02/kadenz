# 01 Installer Kadenz

Ce chapitre explique comment installer Kadenz sur un ordinateur Windows, ce que l'installateur fait et ne fait pas, et comment le mettre à jour, le réparer ou le désinstaller.

## 1.1 Les deux installateurs

Kadenz est livré en deux éditions, avec deux fichiers d'installation :

| Édition | Fichier | Pour qui |
| --- | --- | --- |
| **Kadenz** (un poste) | `KadenzMonoposteSetup-<version>.exe` | une entreprise qui travaille sur un seul ordinateur : le serveur est intégré au poste |
| **Kadenz Entreprise** (multiposte) | `KadenzSetup-<version>.exe` | une entreprise qui relie plusieurs postes à un serveur (chapitre 13) |

`<version>` est le numéro de version, par exemple 0.3.1. Les deux éditions partagent le même mécanisme de mise à niveau : installer l'une remplace l'autre.

> **Attention** Choisissez votre édition avant d'installer. Un serveur multiposte s'installe avec son propre installateur sur la machine du serveur (chapitre 13).

## 1.2 Ce qu'il faut

- un ordinateur sous Windows ;
- le fichier d'installation fourni par l'éditeur ou le revendeur ;
- les droits pour installer un programme sur l'ordinateur : l'installation se fait **pour tout le poste**, dans `Program Files`.

Kadenz est livré en **un seul programme** (`kadenz.exe`) : les polices, les traductions et les images y sont intégrées. Il n'a besoin d'aucune autre bibliothèque.

## 1.3 Installer pas à pas

1. **Lancez le fichier d'installation** (double-clic). Windows peut demander l'autorisation d'installer : acceptez.
2. **Choisissez la langue** de l'assistant d'installation parmi les sept langues de Kadenz : français, anglais, espagnol, turc, arabe, japonais et chinois. Votre choix est retenu pour la réparation et la désinstallation. La page d'accueil est intitulée « Bienvenue dans l'installation de Kadenz » : « Kadenz, logiciel de gestion commerciale, va être installé sur cet ordinateur. » Cliquez sur **Suivant**.
3. **Choisissez le dossier d'installation** (« Dossier d'installation : »). Par défaut, c'est `Program Files\Kadenz`. **Parcourir...** permet d'en choisir un autre.
4. **Cochez les raccourcis** que vous voulez :
   - **Créer un raccourci sur le bureau** ;
   - **Ajouter Kadenz au menu Démarrer**.
   Les deux sont cochés par défaut.
5. Cliquez sur **Installer**. Une page « Installation en cours » montre la progression et le détail des actions. Avec **Précédent**, vous revenez aux choix précédents avant de lancer l'installation ; **Annuler** l'abandonne, après confirmation.
6. À la fin, la page « Installation terminée » indique « Kadenz est installé dans : » suivi du dossier. La case **Lancer Kadenz** est cochée par défaut : cliquez sur **Terminer** pour ouvrir Kadenz tout de suite, avec les droits de votre compte.

Si l'installation échoue, la page « L'installation a échoué » propose d'ouvrir le **journal d'installation** : conservez-le pour le support.

Au premier lancement, Kadenz vous présente l'application, puis l'assistant d'installation crée votre entreprise (chapitre 02).

## 1.4 Ce que l'installation contient

Dans le dossier d'installation :

| Élément | Rôle |
| --- | --- |
| `kadenz.exe` | le programme |
| `LISEZMOI.txt` | un rappel de l'emplacement des données et du dépannage |
| `docs\` | les notes de version et la liste des composants libres utilisés |
| `modeles\` | les fichiers d'exemple pour l'import : `modele-tiers` et `modele-produits`, en CSV et en Excel (chapitre 07) |

## 1.5 Où sont vos données

Vos données ne sont **pas** dans le dossier d'installation. La base de données, sa clé et les journaux de votre compte Windows sont dans le dossier `Kadenz` de votre profil (`%APPDATA%\Kadenz`). Dans l'édition un poste, le serveur intégré et la copie du poste utilisent aussi le dossier `Kadenz` de `%LOCALAPPDATA%`.

L'installation et les mises à jour n'y touchent pas.

## 1.6 Mettre à jour

Lancez simplement le nouvel installateur : il remplace la version précédente et garde vos données. Si Kadenz est ouvert, l'installateur lui demande de se fermer ; vous pouvez aussi utiliser la recherche de mise à jour dans Kadenz (chapitre 12).

> **Astuce** Faites une sauvegarde manuelle avant toute mise à jour (chapitre 11).

Une version plus ancienne ne peut pas remplacer une version plus récente : l'installateur répond « Une version plus récente de Kadenz est déjà installée. »

## 1.7 Réparer ou désinstaller

Relancez le fichier d'installation de la version déjà installée, ou ouvrez la liste des applications de Windows. La page « Que voulez-vous faire ? » propose **Réparer** et **Désinstaller**.

> **Attention** Désinstaller Kadenz supprime toutes ses données sur ce compte Windows : documents, clients et fournisseurs, produits, réglages. Faites d'abord une sauvegarde dans Kadenz. Les sauvegardes déjà faites ne sont pas supprimées, parce qu'elles sont rangées en dehors du dossier des données (chapitre 11).

Dans l'édition un poste, la désinstallation supprime aussi le serveur intégré et la copie du poste.

## 1.8 Installer sans écran

Pour les installations en série, l'installateur accepte ces options en ligne de commande : `/install`, `/repair`, `/uninstall`, `/quiet` (sans aucun écran), `/passive` (avec la barre de progression seule) et `/log fichier` (pour écrire le journal d'installation dans un fichier).

## 1.9 À retenir

- Deux installateurs : Kadenz (un poste) et Kadenz Entreprise (multiposte).
- On installe dans `Program Files\Kadenz`, avec ou sans raccourcis.
- Les données restent dans votre profil Windows et ne sont pas touchées par une mise à jour.
- Désinstaller supprime les données de ce compte : sauvegardez d'abord.

Le chapitre suivant décrit le premier lancement et l'assistant d'installation de l'entreprise.
