//! Feuille de style du portail, servie en `/style.css` : couleurs et polices
//! de la marque (docs/marque/README.md), angles de 4 px au plus, aucune
//! italique, aucune ressource externe.

pub const STYLE: &str = r#"
@font-face { font-family: "Inter"; font-weight: 400; font-style: normal; font-display: swap; src: url("/police/inter-regular.ttf") format("truetype"); }
@font-face { font-family: "Inter"; font-weight: 600; font-style: normal; font-display: swap; src: url("/police/inter-semibold.ttf") format("truetype"); }
@font-face { font-family: "Space Grotesk"; font-weight: 700; font-style: normal; font-display: swap; src: url("/police/space-grotesk-bold.ttf") format("truetype"); }

:root {
  --indigo: #2D3A8C;
  --indigo-survol: #253178;
  --indigo-pale: #E8EAF6;
  --nuit: #1A2259;
  --ambre: #E3A23B;
  --encre: #1F2933;
  --ardoise: #52606D;
  --trait: #D9DCE3;
  --brume: #F4F5F9;
  --succes: #067647;
  --succes-fond: #ECF7F1;
  --erreur: #B42318;
  --erreur-fond: #FDF0EF;
  --clair-sur-nuit: #C5CAE0;
}

* { box-sizing: border-box; }
em, i, cite, var, address, dfn { font-style: normal; }

html { -webkit-text-size-adjust: 100%; scrollbar-gutter: stable; }
body {
  margin: 0;
  background: var(--brume);
  color: var(--encre);
  font-family: "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 15px;
  line-height: 1.5;
}

a { color: var(--indigo); }
a:hover { color: var(--indigo-survol); }
:focus-visible { outline: 2px solid var(--indigo); outline-offset: 2px; }

h1, h2 { font-family: "Space Grotesk", "Inter", system-ui, sans-serif; font-weight: 700; color: var(--encre); letter-spacing: -0.01em; }
h1 { font-size: 26px; margin: 0; line-height: 1.25; }
h2 { font-size: 18px; margin: 28px 0 12px; }

/* Bandeau */
.bandeau { background: var(--nuit); color: #FFFFFF; }
.bandeau-interieur {
  max-width: 1120px; margin: 0 auto; padding: 10px 16px;
  display: flex; align-items: center; gap: 24px; flex-wrap: wrap;
}
.marque { display: flex; align-items: center; gap: 10px; color: #FFFFFF; text-decoration: none; }
.marque:hover { color: #FFFFFF; }
.symbole { width: 28px; height: 28px; display: block; }
.marque-nom { font-family: "Space Grotesk", "Inter", sans-serif; font-weight: 700; font-size: 17px; }
.menu { display: flex; gap: 4px; flex-wrap: wrap; }
.menu-lien { color: var(--clair-sur-nuit); text-decoration: none; padding: 6px 10px; border-radius: 4px; font-weight: 600; }
.menu-lien:hover { color: #FFFFFF; background: rgba(255, 255, 255, 0.08); }
.menu-lien[aria-current="page"] { color: #FFFFFF; background: rgba(255, 255, 255, 0.14); }
.compte { margin-left: auto; display: flex; align-items: center; gap: 12px; }
.compte-nom { color: #FFFFFF; text-decoration: none; font-weight: 600; display: flex; flex-direction: column; line-height: 1.2; text-align: right; }
.compte-nom:hover { color: #FFFFFF; text-decoration: underline; }
.compte-role { color: var(--clair-sur-nuit); font-weight: 400; font-size: 13px; }
.deconnexion { margin: 0; }

/* Contenu */
.contenu { max-width: 1120px; margin: 0 auto; padding: 24px 16px 48px; }
.titre-page { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; margin-bottom: 20px; }
.titre-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.sous-titre { color: var(--ardoise); margin: 4px 0 0; }
.texte { color: var(--ardoise); margin: 0 0 16px; max-width: 760px; }
.vide { color: var(--ardoise); padding: 20px 16px; margin: 0; }

.carte { background: #FFFFFF; border: 1px solid var(--trait); border-radius: 4px; }
.carte-interieur { padding: 20px; }

/* Messages */
.message { border: 1px solid; border-radius: 4px; padding: 10px 14px; margin: 0 0 20px; font-weight: 600; }
.message-ok { color: var(--succes); border-color: var(--succes); background: var(--succes-fond); }
.message-erreur { color: var(--erreur); border-color: var(--erreur); background: var(--erreur-fond); }

/* Boutons */
.bouton {
  display: inline-flex; align-items: center; justify-content: center;
  font: inherit; font-weight: 600; font-size: 14px; line-height: 1.2;
  padding: 9px 16px; border-radius: 4px; border: 1px solid var(--trait);
  background: #FFFFFF; color: var(--encre); text-decoration: none; cursor: pointer;
  white-space: nowrap;
}
.bouton:hover { background: var(--indigo-pale); color: var(--encre); }
.bouton-principal { background: var(--indigo); border-color: var(--indigo); color: #FFFFFF; }
.bouton-principal:hover { background: var(--indigo-survol); border-color: var(--indigo-survol); color: #FFFFFF; }
.bouton-sombre { background: transparent; border-color: rgba(255, 255, 255, 0.35); color: #FFFFFF; padding: 7px 12px; }
.bouton-sombre:hover { background: rgba(255, 255, 255, 0.1); color: #FFFFFF; }
.bouton-petit { padding: 5px 10px; font-size: 13px; }

/* Tableaux */
.tableau-cadre { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; }
th, td { text-align: left; padding: 10px 16px; border-bottom: 1px solid var(--trait); vertical-align: middle; }
th { font-size: 13px; font-weight: 600; color: var(--ardoise); background: var(--brume); white-space: nowrap; }
tbody tr:last-child td { border-bottom: none; }
tbody tr:hover td { background: #FAFBFD; }
td.nombre, th.nombre { text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
td.court { white-space: nowrap; }
.identifiant.identifiant-court { word-break: normal; white-space: nowrap; }
td.details { overflow-wrap: anywhere; min-width: 260px; }
.identifiant { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 13px; color: var(--ardoise); word-break: break-all; }
.etat { display: inline-block; font-size: 13px; font-weight: 600; padding: 2px 8px; border-radius: 4px; border: 1px solid; white-space: nowrap; }
.etat-ok { color: var(--succes); border-color: var(--succes); background: var(--succes-fond); }
.etat-attente { color: var(--indigo); border-color: var(--indigo); background: var(--indigo-pale); }
.etat-neutre { color: var(--ardoise); border-color: var(--trait); background: var(--brume); }

/* Fiche */
.grille-fiche { display: grid; grid-template-columns: minmax(160px, 220px) 1fr; gap: 10px 24px; margin: 0; }
.grille-fiche dt { color: var(--ardoise); }
.grille-fiche dd { margin: 0; font-weight: 600; word-break: break-word; }

/* Formulaires */
.formulaire { max-width: 640px; }
.champ { display: flex; flex-direction: column; gap: 6px; margin-bottom: 16px; }
.champ label, .legende { font-weight: 600; font-size: 14px; }
.aide { color: var(--ardoise); font-size: 13px; margin: 0; }
.rangee { display: grid; grid-template-columns: 1fr 1fr; gap: 0 16px; }
input[type="text"], input[type="password"], input[type="number"], input[type="date"], select, textarea {
  font: inherit; color: var(--encre); background: #FFFFFF;
  border: 1px solid var(--trait); border-radius: 4px; padding: 8px 10px; width: 100%;
}
input:focus, select:focus, textarea:focus { border-color: var(--indigo); outline: 1px solid var(--indigo); }
fieldset { border: 1px solid var(--trait); border-radius: 4px; padding: 12px 16px 4px; margin: 0 0 16px; }
legend { padding: 0 6px; }
.case { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; font-weight: 400; }
.case input { width: 16px; height: 16px; accent-color: var(--indigo); margin: 0; }
.cases { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0 16px; }
.actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 8px; }
.en-ligne { display: inline; margin: 0; }

/* Licence */
.codes-tires { list-style: none; margin: 0 0 12px; padding: 0; display: grid; gap: 6px; }
.code-activation { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 20px; letter-spacing: 0.06em; }
.licence-texte {
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 13px;
  width: 100%; min-height: 140px; resize: vertical; word-break: break-all;
  background: var(--brume);
}

/* Connexion : thème « piliers », adopté par l'utilisateur le 2026-11-19
   (docs/marque/propositions-2026-11-19-portail) : noir pur, vert sauge,
   colonnes fixes en points (image servie par le portail), champs pleins
   arrondis. Code de connexion en six cases par le script du portail. Cadre
   sans ligne de bordure (demande de l'utilisateur du 2026-11-19). Petits
   textes en gris #8E8E93 (systemGray d'Apple, 6,4:1 sur le noir), surtitre en
   sauge ; code faux ou juste montré par la seule bordure des cases, rouge et
   vert d'Apple (proposition C de planche-code-gris, 2026-11-19). */
body.connexion { background: #000000; color: #F2F3EF; padding: 32px; }
body.connexion :focus-visible { outline-color: #A7B58E; }
.cadre-connexion {
  display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);
  max-width: 1360px; min-height: calc(100vh - 64px); margin: 0 auto; padding: 10px;
  background: #000000; border: 0;
}
.colonne-connexion { display: flex; align-items: center; justify-content: center; padding: 56px 40px; }
.connexion-boite { width: 100%; max-width: 380px; }
.connexion-symbole { display: flex; justify-content: center; margin-bottom: 44px; }
.connexion-symbole .symbole { width: 52px; height: 52px; }
.surtitre { color: #A7B58E; font-weight: 600; font-size: 12px; letter-spacing: 0.12em; text-transform: uppercase; margin: 0 0 10px; }
body.connexion h1 { font-family: "Inter", system-ui, sans-serif; font-weight: 600; font-size: 28px; color: #F2F3EF; margin: 0 0 10px; }
body.connexion .texte { color: #8E8E93; font-size: 14px; line-height: 1.6; margin: 0 0 32px; }
body.connexion .champ { gap: 8px; margin-bottom: 18px; }
body.connexion .champ label { color: #8E8E93; font-size: 13px; }
.saisie { position: relative; }
.saisie svg {
  position: absolute; left: 15px; top: 50%; width: 18px; height: 18px; transform: translateY(-50%);
  fill: none; stroke: #8E8E93; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; pointer-events: none;
}
body.connexion input[type="text"], body.connexion input[type="password"] {
  height: 48px; padding: 0 16px 0 44px; font-size: 15px; color: #F2F3EF;
  background: #17181A; border: 1px solid #17181A; border-radius: 9px;
}
body.connexion input:hover { background: #1D1F21; border-color: #2A2C2E; }
body.connexion input:focus { outline: none; background: #1D1F21; border-color: #A7B58E; box-shadow: 0 0 0 4px rgba(167, 181, 142, 0.16); }
.bouton-sauge {
  display: flex; align-items: center; justify-content: center; width: 100%; min-height: 48px; margin-top: 18px;
  font: inherit; font-size: 15px; font-weight: 600; color: #0E1209; background: #A7B58E;
  border: 0; border-radius: 9px; cursor: pointer;
}
.bouton-sauge:hover { background: #BCC9A4; }
.bouton-sauge:focus-visible { outline: 2px solid #BCC9A4; outline-offset: 3px; }
body.connexion .message-erreur { color: #F08A8A; background: rgba(229, 83, 75, 0.1); border-color: rgba(229, 83, 75, 0.38); border-radius: 9px; }
.connexion-retour { margin: 20px 0 0; text-align: center; font-size: 14px; }
.connexion-retour a { color: #8E8E93; text-decoration: none; }
.connexion-retour a:hover { color: #F2F3EF; }
.separateur { display: flex; align-items: center; gap: 12px; margin: 32px 0 16px; font-size: 12px; color: #8E8E93; }
.separateur::before, .separateur::after { content: ""; flex: 1; height: 1px; background: #2A2C2E; }
.connexion-pied { margin: 0; font-size: 13px; line-height: 1.6; text-align: center; color: #8E8E93; }
.illustration-connexion { position: relative; min-height: 520px; overflow: hidden; border-radius: 14px; background: #000000; }
.illustration-connexion img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center bottom; }
body.connexion [hidden] { display: none !important; }
/* Code en six cases (statique/code-connexion-*.js) */
.chiffres-code { min-width: 0; margin: 0 0 4px; padding: 0; border: 0; border-radius: 0; }
.chiffres-code legend { margin-bottom: 10px; padding: 0; font-size: 13px; font-weight: 500; color: #8E8E93; }
.chiffres { display: flex; align-items: center; gap: 8px; }
body.connexion .chiffres input[type="text"] {
  flex: 1 1 0; width: 0; min-width: 0; height: 58px; padding: 0; text-align: center;
  font-size: 24px; font-weight: 600; font-variant-numeric: tabular-nums; caret-color: #A7B58E;
  border-width: 1.5px; transition: border-color 0.15s ease, box-shadow 0.15s ease, background-color 0.2s ease;
}
.chiffres-tiret { flex: 0 0 10px; height: 2px; background: #2A2C2E; }
.chiffres-code.verification input { border-color: #A7B58E; animation: code-attente 0.9s ease-in-out infinite; }
body.connexion .chiffres-code.juste input { background: #17181A; border-color: #30D158; box-shadow: none; }
body.connexion .chiffres-code.faux input { background: #17181A; border-color: #FF453A; box-shadow: none; }
.chiffres-code.faux .chiffres { animation: code-secousse 0.42s ease-in-out; }
@keyframes code-attente { 0%, 100% { background: #17181A; } 50% { background: #23261F; } }
@keyframes code-secousse {
  0%, 100% { transform: translateX(0); } 20% { transform: translateX(-7px); } 40% { transform: translateX(6px); }
  60% { transform: translateX(-4px); } 80% { transform: translateX(2px); }
}
/* Avis du code : lu par les lecteurs d'écran, invisible à l'écran. */
.avis-code { position: absolute; width: 1px; height: 1px; margin: -1px; padding: 0; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
.bascule-code { margin: 20px 0 0; text-align: center; }
.lien-code {
  padding: 0; font: inherit; font-size: 14px; color: #8E8E93; background: none; border: 0; cursor: pointer;
}
.lien-code:hover { color: #F2F3EF; }
@media (prefers-reduced-motion: reduce) {
  .chiffres-code.verification input, .chiffres-code.faux .chiffres { animation: none; }
  body.connexion .chiffres input[type="text"] { transition: none; }
}

/* Pages du compte au thème piliers (2026-11-20) : erreur, double
   authentification, mot de passe. Mêmes champs, bouton sauge et couleurs que
   la connexion ; clé et codes de secours à chasse fixe sur fond de champ. */
.connexion-boite-large { max-width: 460px; }
a.bouton-sauge, a.bouton-sauge:hover { color: #0E1209; text-decoration: none; }
body.connexion input[type="text"].sans-icone { padding-left: 16px; }
body.connexion input.code-activation[type="text"] { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 15px; letter-spacing: 0.04em; }
body.connexion .aide { color: #8E8E93; font-size: 13px; line-height: 1.6; }
body.connexion .texte strong { color: #F2F3EF; font-weight: 600; }
body.connexion .message-ok { color: #30D158; background: rgba(48, 209, 88, 0.1); border-color: rgba(48, 209, 88, 0.38); border-radius: 9px; }
body.connexion .codes-tires { list-style: none; display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin: 0 0 8px; padding: 0; }
body.connexion .codes-tires li { padding: 12px 4px; text-align: center; background: #17181A; border-radius: 9px; }
body.connexion .codes-tires .code-activation { font-size: 15px; letter-spacing: 0.04em; color: #F2F3EF; }
.pied-compte { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 8px 24px; margin: 20px 0 0; font-size: 14px; }
.pied-compte a { color: #8E8E93; text-decoration: none; }
.pied-compte a:hover { color: #F2F3EF; }

@media (max-width: 860px) {
  body.connexion { padding: 12px; }
  .cadre-connexion { grid-template-columns: 1fr; min-height: calc(100vh - 24px); }
  .illustration-connexion { display: none; }
  .colonne-connexion { padding: 40px 20px; }
}
/* Clé de la double authentification (8 groupes) entière sur un téléphone. */
@media (max-width: 480px) {
  body.connexion input.code-activation[type="text"] { padding: 0 10px; font-size: 11.5px; letter-spacing: 0; }
}

@media (max-width: 760px) {
  .rangee { grid-template-columns: 1fr; }
  .grille-fiche { grid-template-columns: 1fr; gap: 2px; }
  .grille-fiche dd { margin-bottom: 10px; }
  .compte { margin-left: 0; width: 100%; justify-content: space-between; }
  .compte-nom { text-align: left; }
  th, td { padding: 8px 10px; }
}

/* Console : pages d'un compte connecté au thème de la connexion, noir
   profond et vert sauge (demande de l'utilisateur du 2026-11-21, « mêmes
   couleurs, sans la limite, tout fluide »). Page pleine largeur, en-tête en
   pilules, cartes #17181A, champs pleins, pied ; pages à formulaire en écran
   partagé avec les colonnes de la connexion à gauche. Tout est rangé sous
   body.console : la connexion et les pages piliers ne changent pas. */
body.console { background: #000000; color: #F2F3EF; color-scheme: dark; overflow-x: clip; min-height: 100vh; display: flex; flex-direction: column; }
body.console :focus-visible { outline: 2px solid #A7B58E; outline-offset: 2px; }
body.console ::selection { background: #A7B58E; color: #0E1209; }
body.console a { color: #F2F3EF; text-decoration-color: rgba(242, 243, 239, 0.32); text-underline-offset: 3px; }
body.console a:hover { color: #A7B58E; text-decoration-color: currentColor; }
body.console h1, body.console h2 { font-family: "Inter", system-ui, sans-serif; color: #F2F3EF; }
body.console h1 { font-weight: 400; font-size: clamp(34px, 4.2vw, 60px); line-height: 1.05; letter-spacing: -0.03em; }
body.console h2 { font-weight: 600; font-size: 18px; letter-spacing: -0.01em; margin: 40px 0 14px; }
body.console strong { color: #F2F3EF; font-weight: 600; }

/* Boutons */
body.console .bouton { border: 0; border-radius: 999px; padding: 10px 18px; background: #17181A; color: #F2F3EF; text-decoration: none; transition: background-color 0.2s ease, color 0.2s ease; }
body.console .bouton:hover { background: #2A2C2E; color: #F2F3EF; }
body.console .bouton-principal { background: #A7B58E; color: #0E1209; }
body.console .bouton-principal:hover { background: #BCC9A4; color: #0E1209; }
body.console .bouton-petit { padding: 6px 12px; font-size: 13px; }

/* Contenu pleine largeur */
body.console .contenu { flex: 1 0 auto; width: 100%; max-width: none; margin: 0; padding: 8px clamp(16px, 3vw, 40px) 64px; }
body.console .titre-page { align-items: flex-end; margin: clamp(28px, 5vw, 72px) 0 clamp(24px, 3vw, 40px); }
body.console .sous-titre, body.console .texte, body.console .vide, body.console .aide { color: #8E8E93; }
body.console .texte { font-size: 15px; line-height: 1.6; max-width: 720px; }
body.console .carte { background: #17181A; border: 0; border-radius: 14px; overflow: hidden; }
body.console .carte-interieur { padding: clamp(18px, 2.4vw, 28px); }
body.console .message { border-radius: 9px; font-weight: 500; color: #F2F3EF; }
body.console .message-ok { border-color: #30D158; background: rgba(48, 209, 88, 0.1); }
body.console .message-erreur { border-color: #FF453A; background: rgba(255, 69, 58, 0.1); }

/* Tableaux */
body.console th, body.console td { border-bottom: 1px solid #2A2C2E; padding: 14px 18px; }
body.console th { background: transparent; color: #8E8E93; font-weight: 500; }
body.console tbody tr { transition: background-color 0.2s ease; }
body.console tbody tr:hover td { background: #1D1F21; }
body.console .identifiant { color: #8E8E93; }
body.console .etat { border-radius: 999px; padding: 3px 10px; font-weight: 500; }
body.console .etat-ok { color: #30D158; border-color: rgba(48, 209, 88, 0.38); background: rgba(48, 209, 88, 0.1); }
body.console .etat-attente { color: #A7B58E; border-color: rgba(167, 181, 142, 0.4); background: rgba(167, 181, 142, 0.1); }
body.console .etat-neutre { color: #8E8E93; border-color: #2A2C2E; background: transparent; }

/* Fiches et formulaires */
body.console .grille-fiche dt { color: #8E8E93; }
body.console .grille-fiche dd { color: #F2F3EF; font-weight: 500; }
body.console .champ { gap: 8px; margin-bottom: 18px; }
body.console .champ label, body.console .legende { color: #8E8E93; font-size: 13px; font-weight: 500; }
body.console input[type="text"], body.console input[type="password"], body.console input[type="number"], body.console input[type="date"], body.console select, body.console textarea {
  min-height: 48px; padding: 0 16px; font-size: 15px; color: #F2F3EF;
  background: #1D1F21; border: 1px solid #1D1F21; border-radius: 9px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background-color 0.15s ease;
}
body.console textarea { padding: 14px 16px; }
body.console input:hover, body.console select:hover, body.console textarea:hover { border-color: #2A2C2E; }
body.console input:focus, body.console select:focus, body.console textarea:focus { outline: none; border-color: #A7B58E; box-shadow: 0 0 0 4px rgba(167, 181, 142, 0.16); }
body.console fieldset { border: 1px solid #2A2C2E; border-radius: 9px; }
body.console .case input { accent-color: #A7B58E; min-height: 0; }
body.console .code-activation { color: #F2F3EF; }
body.console .licence-texte { background: #1D1F21; }
body.console .actions { margin-top: 18px; }
body.console .formulaire > p.aide { margin-bottom: 18px; }

/* Écran partagé des pages à formulaire : contenu à plat, sans carte. */
body.console .contenu-partage { display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 6fr); gap: clamp(24px, 5vw, 88px); align-items: start; padding-top: 8px; }
body.console .panneau-illustration { position: sticky; top: 80px; height: calc(100vh - 104px); min-height: 480px; overflow: hidden; border-radius: 14px; background: #000000; }
body.console .panneau-illustration img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center bottom; }
body.console .colonne-formulaire { min-width: 0; max-width: 620px; padding-bottom: 24px; }
body.console .colonne-formulaire .titre-page { margin-top: clamp(24px, 6vh, 72px); }
body.console .colonne-formulaire h1 { font-size: clamp(30px, 3vw, 44px); }
body.console .colonne-formulaire .carte { background: transparent; border-radius: 0; overflow: visible; }
body.console .colonne-formulaire .carte-interieur { padding: 0; }
body.console .colonne-formulaire input[type="text"], body.console .colonne-formulaire input[type="number"], body.console .colonne-formulaire input[type="date"], body.console .colonne-formulaire select, body.console .colonne-formulaire textarea { background: #17181A; border-color: #17181A; }
body.console .colonne-formulaire input:focus, body.console .colonne-formulaire select:focus, body.console .colonne-formulaire textarea:focus { background: #1D1F21; border-color: #A7B58E; }
body.console .colonne-formulaire .bouton-principal { min-height: 48px; padding: 0 24px; border-radius: 9px; }
body.console .colonne-formulaire .codes-tires li { padding: 12px 16px; background: #17181A; border-radius: 9px; }

/* Coque animée (gabarit, 2026-11-21) : l'en-tête est posé au-dessus du
   contenu ; panneau des pages à formulaire animé (C Tournant et pierre). */
body.console .contenu { padding-top: calc(var(--nav-h, 64px) + 40px); }
body.console .panneau-illustration { background: #000000; }
body.console .panneau-illustration canvas { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
body.console .panneau-fond { opacity: 0.45; }
body.console .footer { margin-top: clamp(64px, 10vw, 140px); }
/* Page Entreprises au gabarit plein écran : sections de bord à bord, puis la
   liste complète ; le message éventuel passe au-dessus du diaporama. */
body.console main.pleine { flex: 1 0 auto; width: 100%; }
/* Étape C : étiquettes du gabarit en haut des pages, état vide illustré. */
body.console .etiquettes-page { margin: 0 0 clamp(24px, 4vw, 48px); color: rgba(255, 255, 255, 0.56); }
body.console .vide-etat { display: flex; align-items: center; gap: 16px; padding: 20px 16px; }
body.console .vide-etat canvas { width: 56px; height: 56px; flex: none; display: block; }
body.console .vide-etat .vide { padding: 0; }
/* Boutons et champs du gabarit : police de la page (Inter). Bouton animé
   (dérive du code d'activation) : texte centré dans sa case, que la couche
   de survol élargit, et même graisse que les autres boutons sauge. */
body.console main :is(button, input, select, textarea) { font-family: inherit; }
body.console .button-animated, body.console .button-animated::after { font-family: inherit; font-weight: 600; }
body.console .button-animated { color: #0E1209; }
body.console .button-animated-inner { justify-content: center; text-align: center; }
body.console .bloc-formulaire { align-items: stretch; padding-top: 0; margin: clamp(40px, 6vw, 72px) 0 clamp(32px, 5vw, 56px); }
body.console .bloc-formulaire .panneau-illustration { position: relative; top: auto; height: auto; min-height: 440px; }
body.console .bloc-formulaire .colonne-formulaire h2 { margin-top: 0; }
@media (max-width: 860px) {
  body.console .bloc-formulaire .panneau-illustration { min-height: 0; height: 180px; }
}
body.console .contenu-message { position: relative; z-index: 5; padding-bottom: 0; }
body.console .liste-entreprises { padding: clamp(64px, 8vw, 120px) clamp(16px, 3vw, 40px) 0; }
body.console .liste-tete { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 12px; }
body.console .liste-tete h2 { margin: 0; font-size: clamp(28px, 3.2vw, 44px); font-weight: 500; letter-spacing: -0.02em; }

@media (max-width: 860px) {
  body.console .contenu-partage { grid-template-columns: 1fr; gap: 24px; }
  body.console .panneau-illustration { position: relative; top: auto; height: 180px; min-height: 0; }
  body.console .colonne-formulaire { max-width: none; }
  body.console .colonne-formulaire .titre-page { margin-top: 8px; }
}
@media (max-width: 480px) {
  body.console .titre-page { align-items: flex-start; }
}
"#;
