# 14 L'accès à distance

L'accès à distance permet à un poste de travailler **hors du bureau** (à la maison, en déplacement) en rejoignant le serveur de l'entreprise. Ce chapitre explique ce qu'il faut, comment l'activer, comment choisir les postes autorisés et ce qu'on voit en travaillant à distance.

> **Attention** L'accès à distance n'existe que dans l'édition **multiposte**, sur un poste relié à un serveur. L'édition Kadenz un poste n'a pas cette rubrique : son serveur est intégré au poste lui-même.

## 14.1 Le principe

Un poste hors du bureau ne peut pas joindre directement le serveur de l'entreprise. Il passe par un **relais** exploité par l'éditeur. Kadenz affiche ces garanties dans la rubrique :

- les échanges sont **chiffrés de bout en bout** : l'éditeur ne peut pas les lire ;
- le relais **ne garde aucune donnée** ;
- si le serveur de l'entreprise est éteint, un poste distant travaille **hors connexion** (chapitre 13).

## 14.2 Ce qu'il faut avant de commencer

1. **Un relais réglé sur le serveur.** L'adresse et l'empreinte du relais vous sont données par l'éditeur. On les enregistre une fois dans la fenêtre d'administration du serveur, section **Relais d'accès à distance** (chapitre 13, section 13.4). Sans relais réglé, la page affiche : « Aucun relais n'est réglé dans la fenêtre du serveur : l'accès à distance n'est pas encore disponible. »
2. **Un poste relié**, c'est-à-dire déjà inscrit à l'entreprise (chapitre 13, section 13.2), et **connecté au serveur**. Hors connexion, la page indique : « Disponible seulement quand le poste est connecté au serveur. »
3. **Un compte Admin.** La rubrique n'apparaît dans les **Paramètres** que pour un compte du rôle Admin.

## 14.3 Activer l'accès à distance

Ouvrez **Paramètres**, rubrique **Accès à distance**. La page lit d'abord l'état sur le serveur (« Lecture de l'état sur le serveur... »), puis propose :

- la case **Activer l'accès à distance pour l'entreprise**. Elle est **décochée par défaut** : tant qu'elle ne l'est pas, aucun poste ne peut passer par le relais ;
- la zone **État du relais** ;
- la liste **Postes autorisés à distance**.

### L'état du relais

| Ligne | Contenu |
| --- | --- |
| **Relais** | l'adresse du relais, ou **Aucun** |
| **État** | **Accès désactivé**, **Aucun relais réglé**, **Joignable depuis le serveur** (affiché en vert) ou **Injoignable depuis le serveur** |
| **Dernière connexion distante** | la date et l'heure de la dernière connexion d'un poste par le relais, ou **Aucune** |

Quand l'état indique **Injoignable depuis le serveur**, le serveur de l'entreprise n'arrive pas à joindre le relais : vérifiez d'abord que la machine du serveur a accès à Internet. Tant que l'état n'est pas **Joignable**, aucun poste distant ne peut se connecter.

### Choisir les postes autorisés

Sous **Postes autorisés à distance**, chaque poste inscrit a sa propre case, **décochée par défaut**. « Seuls les postes cochés peuvent passer par le relais. » Le but est que l'accès à distance reste réservé aux ordinateurs que vous avez choisis, par exemple le portable du gérant.

Si aucun poste n'est inscrit, la page affiche « Aucun poste inscrit. »

> **Attention** Décocher un poste **ferme aussitôt ses sessions distantes**. C'est la bonne réaction en cas de perte ou de vol d'un ordinateur. Les cases se règlent aussi quand l'accès à distance est désactivé.

Pour retirer définitivement un poste de l'entreprise, utilisez **Retirer** dans la fenêtre d'administration du serveur (chapitre 13).

## 14.4 Travailler à distance

Un poste autorisé et connecté à Internet se relie au serveur de lui-même. L'indicateur de connexion, en haut à droite de la fenêtre, le montre :

- **Connecté à distance** : le poste est relié par le relais ;
- **Synchronisation à distance...**, avec un pourcentage : le poste met sa copie à jour ;
- **Connecté à distance, lecture seule** : l'abonnement est échu, la saisie est bloquée (chapitre 12) ;
- **Hors connexion** : la liaison est perdue ; le travail continue sur la copie du poste et attend dans la file d'envoi (chapitre 13, section 13.5).

Vous travaillez ensuite comme au bureau. Une validation de document demande le serveur, qui attribue le numéro.

## 14.5 Quelques réflexes

- Autorisez seulement les postes qui en ont besoin.
- Décochez un poste dès qu'il n'a plus à passer par le relais (ordinateur perdu, départ d'un collaborateur).
- Si l'état reste **Injoignable depuis le serveur** après vérification d'Internet, contactez l'éditeur avec l'adresse du relais affichée à la page.

## 14.6 À retenir

- L'accès à distance existe dans l'édition multiposte, et se règle depuis un compte Admin.
- Il faut un relais réglé sur le serveur, l'accès activé pour l'entreprise, et le poste coché dans la liste.
- Les échanges sont chiffrés de bout en bout : l'éditeur ne lit rien.
- Décocher un poste ferme ses sessions aussitôt.
- L'indicateur affiche **Connecté à distance** quand tout va bien.

Le chapitre suivant rassemble les réponses aux problèmes courants et les annexes.
