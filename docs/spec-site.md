# Kadenz phase 2 - Site public (chantier 3.7)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Brouillon de kadenz-a8 après les décisions 70 à 73 de l'utilisateur : pages, adresses, liste SEO, espace client, contact, jalons, ce que l'utilisateur fournit. Pile technique et calendrier laissés à discuter avec l'utilisateur. Relecture de kadenz-22 prévue après l'essai S3. Rien n'est codé. |

Statut : **proposé**. Les décisions citées sont celles du cahier de la phase 2 (`docs/cahier-phase2.md`, section 2 et section 3.7). Ce qui est marqué « à choisir » attend l'utilisateur ; ce qui est marqué « proposé » est une proposition de kadenz-a8, révisable. Aucun texte, aucun chiffre et aucune donnée structurée ne seront inventés pour remplir une page.

## 1. Décisions de départ

- **70. Contenu** : présentation de Kadenz, téléchargements, espace client, contact et demande de démonstration, et « tout le nécessaire » (mentions légales, confidentialité, page introuvable).
- **71. Langues** : français d'abord ; le site est prévu multilingue dès le départ.
- **72. Nom de domaine** : choisi plus tard ; le site se prépare en local.
- **73. Tarifs** : pas de page de tarifs tant que la grille de prix n'est pas fixée (question ouverte 2).
- **Hébergement** (cahier 3.7, décision 16) : sur le VPS de l'éditeur, dans des conteneurs, à côté du relais d'accès à distance. Prestataire du VPS à choisir.
- **Données des entreprises** (décisions 16 et 28) : jamais sur le VPS. L'espace client se limite à l'abonnement, aux licences et aux téléchargements.

## 2. Questions ouvertes

1. **Pile technique** (à choisir, à discuter avec l'utilisateur). Pistes relevées, sans décision :
   - **A. Pages statiques et portail** (recommandée par kadenz-a8) : pages publiques générées en HTML statique par Zola (générateur en un seul binaire Rust, contenus en Markdown, multilingue et plan du site intégrés), servies par Caddy sur le VPS ; espace client et formulaire de contact ajoutés au portail Rust (`kadenz-portail`). Raisons : vitesse et SEO simples à tenir avec des pages statiques, console des licences séparée du trafic public, une seule langue de programmation dans le projet.
   - **B. Tout dans le portail** : pages publiques servies par `kadenz-portail` (axum, maud), comme la console. Un seul programme, pile déjà connue des sessions ; mais pages publiques et console partagent le même processus et la même surface d'attaque, et chaque balise SEO est écrite à la main.
   - **C. Astro** : générateur statique JavaScript, répandu et riche en composants ; ajoute Node.js à un projet tout en Rust.
   - **D. Framework JavaScript complet** (Next.js ou équivalent) : le plus souple et le plus lourd ; contraire à l'orientation « sans JavaScript lourd » de la décision 61, et un serveur Node à exploiter.
2. **Calendrier** (à choisir) : avant la 1.0.0, en parallèle, ou après. L'utilisateur a répondu « après on verra ».
3. **Contenus** : textes, captures, identité de l'éditeur (section 8).
4. **Messages du formulaire de contact** : où ils arrivent (adresse email, service tiers, ou consultation dans le portail), section 6.
5. **Espace client** : qui reçoit un compte et comment (section 5).

## 3. Ce que le site peut annoncer

- Le site ne présente comme disponible que ce qui est publié. Aujourd'hui, seule la version 0.2.2 (pont V1) est publiée ; le multiposte, le serveur, l'accès à distance, le stock et l'export SYSCOHADA sont codés et essayés à l'écran, mais pas publiés. Tant que la 1.0.0 n'est pas sortie, ces fonctions sont soit absentes du site, soit présentées comme à venir, au choix de l'utilisateur.
- La présentation existante (`docs/presentation/kadenz-presentation.html`) date de la V1 : sa section « La suite : plusieurs postes, un serveur » décrit ce qui est maintenant fait. Elle sert de source à mettre à jour avec l'accord de l'utilisateur, pas de texte à recopier.
- Les fonctions citées sont relevées dans le cahier des charges, les cahiers et le code ; chaque affirmation du site doit pouvoir être montrée dans l'application.
- Marque : logo, symbole, couleurs (Indigo, Nuit, Ambre) et texte en ABC Diatype de `docs/marque/README.md` (le mot du logo est vectorisé). Aucune italique, angles de 4 px au plus (préférence de l'utilisateur pour les interfaces web).

## 4. Pages et adresses

Adresses courtes, en minuscules, sans accents, préfixées par la langue dès le départ (`/fr/...`) pour que les autres langues s'ajoutent sans changer les adresses françaises ; la racine `/` renvoie vers `/fr/` (proposé).

| Page | Adresse (proposée) | Contenu | Indexée |
|---|---|---|---|
| Accueil | `/fr/` | ce qu'est Kadenz, pour qui, les modules, appel vers démonstration et téléchargement | oui |
| Fonctions | `/fr/fonctions/` | documents, règlements et relances, tableau de bord, PDF et Excel, langues de l'interface ; multiposte, stock, export comptable selon la section 3 | oui |
| Sécurité | `/fr/securite/` | base chiffrée, sauvegardes chiffrées, mises à jour signées ; seulement ce qui est publié | oui |
| Téléchargements | `/fr/telechargements/` | installateur du poste et paquet du serveur de la dernière version, configuration requise, notes de version | oui |
| Contact et démonstration | `/fr/contact/` | formulaire et coordonnées fournies par l'utilisateur | oui |
| Mentions légales | `/fr/mentions-legales/` | identité de l'éditeur, hébergeur | oui |
| Confidentialité | `/fr/confidentialite/` | données collectées par le formulaire et l'espace client, durée, droits | oui |
| Page introuvable | (réponse 404) | lien vers l'accueil | non |
| Espace client | `/espace/` | voir section 5 | non |

Pas de page de tarifs (décision 73). Pas de blog ni d'actualités dans la première version (proposé : rien à y mettre sans contenu réel).

### Téléchargements

- Fichiers publiés sur les releases GitHub du dépôt : `KadenzSetup-X.Y.Z.exe` (poste) et `KadenzServeur-X.Y.Z.msi` (serveur), noms relevés dans `scripts/release.ps1` et `crates/maj/src/lib.rs`. Le nom contient la version : le lien de la page suit la dernière release (proposé : tiré du manifeste `v2` des mises à jour de F10 à la construction du site, avec l'empreinte SHA-256 affichée à côté de chaque fichier).
- Aucune release n'est publiée sans l'accord de l'utilisateur ; la page ne fait que suivre ce qui est publié.

## 5. Espace client

- Travail nouveau : le portail (`docs/spec-portail.md`) n'a que les rôles éditeur et revendeur. L'espace client demande un rôle « client » rattaché à une entreprise, une façon de créer ses comptes (proposé : par l'éditeur ou le revendeur de l'entreprise, avec un mot de passe à changer à la première connexion), et le portail en service sur le VPS, qui reste à la décision de l'utilisateur.
- Contenu (proposé) : licences de l'entreprise (texte à copier dans la rubrique « Abonnement », échéance, postes, modules), paiements enregistrés, liens de téléchargement. Jamais de données commerciales de l'entreprise (décisions 16 et 28).
- Sécurité : celle du portail (section 6 de `spec-portail.md`) : argon2id, cookie de session `HttpOnly`, `Secure`, `SameSite=Strict`, jeton anti-falsification, connexions freinées, journal.
- Hors SEO : `noindex` sur chaque page, en-tête `X-Robots-Tag: noindex`, chemin `/espace/` interdit dans `robots.txt`, absent du plan du site.

## 6. Contact et demande de démonstration

- Champs (proposé) : nom, entreprise, pays, téléphone ou email (l'un des deux), message, case « demande de démonstration ». Rien d'autre que ce qui sert à répondre.
- Réception : à choisir par l'utilisateur (adresse email, service tiers, ou messages gardés dans le portail et consultés par l'éditeur). Un envoi d'email dépend d'un prestataire et d'un compte externe : décision de l'utilisateur.
- Anti-spam (proposé) : champ piège invisible, nombre d'envois freiné par adresse, aucun captcha d'un tiers (pas de traceur sur le site).
- Confirmation à l'écran après l'envoi ; le texte de la page Confidentialité décrit ces données, leur durée de conservation et la façon d'en demander l'effacement (durée à fixer par l'utilisateur).
- Formulaire et réception sont dynamiques : avec la piste A, une route du portail, sous le même domaine.

## 7. Liste SEO (CLAUDE.md global de l'utilisateur)

Chaque point s'applique aux pages publiques ; l'espace client en est exclu (`noindex`). Vérification au jalon W4, puis à chaque changement notable.

| N° | Point | Application | Vérification |
|---|---|---|---|
| 1 | `sitemap.xml` | généré à la construction, toutes les pages publiques et toutes les langues | lecture du fichier, URL absolues une fois le domaine choisi |
| 2 | `robots.txt` | autorise le site, interdit `/espace/` et les routes du portail, pointe vers le plan du site | lecture du fichier |
| 3 | Aucun `noindex` oublié | seul l'espace client en porte | parcours des pages publiées, en production et pas seulement en local |
| 4 | Canonique et `hreflang` | balise canonique sur chaque page ; `hreflang` (dont `x-default`) dès qu'une deuxième langue existe | lecture du HTML ; URL absolues après le choix du domaine (décision 72) |
| 5 | `<title>` unique | environ 60 caractères, un par page | liste des titres, sans doublon |
| 6 | Meta description unique | environ 150 caractères, tirée du contenu réel de la page | liste des descriptions, sans doublon |
| 7 | Ordre des titres | h1, puis h2, puis h3, sans saut | contrôle automatique du HTML |
| 8 | Un seul `h1` | un par page | contrôle automatique du HTML |
| 9 | Données structurées JSON-LD | `Organization` quand l'identité de l'éditeur est fournie, `SoftwareApplication` pour Kadenz, `BreadcrumbList` sur les pages internes ; jamais de valeur inventée (ni prix, ni note, ni avis) | outil de test des résultats enrichis de Google |
| 10 | Maillage interne | chaque page importante reçoit des liens d'autres pages (menu, pied de page, appels) | carte des liens à la construction |
| 11 | Aucun lien cassé | interne et externe | parcours complet du site construit |
| 12 | Images | formats modernes (WebP ou AVIF), `srcset`, dimensions écrites, chargement différé hors du premier écran ; logos en SVG | contrôle à la construction, poids des pages |
| 13 | Core Web Vitals | LCP, INP, CLS dans le vert sur mobile ; police de la marque hébergée sur le site, aucun script tiers | Lighthouse en profil mobile, puis données réelles de la Search Console |
| 14 | Mobile | pas de débordement ni de texte illisible ; marge latérale de 16 px | essai à 360 px de large |
| 15 | Adresses | courtes, minuscules, sans accents (section 4) | liste des adresses |
| 16 | Open Graph et Twitter | `og:title`, `og:description`, `og:image` (1200 x 630, tirée de la marque), `twitter:card` sur chaque page partageable | aperçu de partage |
| 17 | Search Console | domaine vérifié et plan du site soumis par l'utilisateur, avec son compte ; kadenz-a8 prépare les étapes et le fichier ou l'enregistrement DNS demandé, sans les faire à sa place | faite par l'utilisateur |

## 8. Ce que l'utilisateur fournit

- Les textes, ou l'accord pour partir de la présentation existante mise à jour et de captures de l'application (prises sur un banc d'essai, jamais sur des données réelles).
- L'identité de l'éditeur pour les mentions légales : nom ou raison sociale, forme, adresse, contact, numéros d'immatriculation s'il y en a, directeur de la publication.
- Le choix de la réception des messages du formulaire et la durée de conservation de ces messages.
- Le prestataire du VPS et le nom de domaine, au moment de la mise en ligne.
- Le choix de la pile technique (section 2) et du calendrier.

## 9. Jalons (proposés, après le choix de la pile)

| Jalon | Contenu | Qui |
|---|---|---|
| W1 | Gabarits, style de la marque, adresses multilingues, plan du site, `robots.txt`, page introuvable ; pages construites en local avec les seuls contenus fournis ou approuvés | kadenz-a8, relecture kadenz-22 |
| W2 | Téléchargements tirés des releases publiées, formulaire de contact et sa réception | kadenz-a8 et kadenz-22 |
| W3 | Espace client dans le portail : rôle client, comptes, licences, paiements | kadenz-a8 (portail), kadenz-22 (pages) |
| W4 | Essai à l'écran en local (bureau et mobile) et contrôle des 17 points de la liste SEO | l'utilisateur, avec l'accord pour l'écran |
| W5 | Mise en ligne sur le VPS (conteneurs, Caddy), domaine, Search Console | l'utilisateur décide ; sessions préparent |

Chaque jalon se termine comme ceux de la phase 2 : vérification visible, commit poussé sur `main`, ligne au journal. Une seule session compile à la fois.
