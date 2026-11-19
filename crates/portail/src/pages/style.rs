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
   arrondis. Aucun script. */
body.connexion { background: #000000; color: #F2F3EF; padding: 32px; }
body.connexion :focus-visible { outline-color: #A7B58E; }
.cadre-connexion {
  display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);
  max-width: 1360px; min-height: calc(100vh - 64px); margin: 0 auto; padding: 10px;
  background: #000000; border: 1px solid #1A1A1A; border-radius: 20px;
}
.colonne-connexion { display: flex; align-items: center; justify-content: center; padding: 56px 40px; }
.connexion-boite { width: 100%; max-width: 380px; }
.connexion-symbole { display: flex; justify-content: center; margin-bottom: 44px; }
.connexion-symbole .symbole { width: 52px; height: 52px; }
.surtitre { color: #A7B58E; font-weight: 600; font-size: 12px; letter-spacing: 0.12em; text-transform: uppercase; margin: 0 0 10px; }
body.connexion h1 { font-family: "Inter", system-ui, sans-serif; font-weight: 600; font-size: 28px; color: #F2F3EF; margin: 0 0 10px; }
body.connexion .texte { color: #9DA197; font-size: 14px; line-height: 1.6; margin: 0 0 32px; }
body.connexion .champ { gap: 8px; margin-bottom: 18px; }
body.connexion .champ label { color: #F2F3EF; font-size: 13px; }
.saisie { position: relative; }
.saisie svg {
  position: absolute; left: 15px; top: 50%; width: 18px; height: 18px; transform: translateY(-50%);
  fill: none; stroke: #9DA197; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; pointer-events: none;
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
.connexion-retour a { color: #F2F3EF; text-decoration-color: #9DA197; text-underline-offset: 3px; }
.connexion-retour a:hover { color: #FFFFFF; }
.separateur { display: flex; align-items: center; gap: 12px; margin: 32px 0 16px; font-size: 12px; color: #9DA197; }
.separateur::before, .separateur::after { content: ""; flex: 1; height: 1px; background: #2A2C2E; }
.connexion-pied { margin: 0; font-size: 13px; line-height: 1.6; text-align: center; color: #9DA197; }
.illustration-connexion { position: relative; min-height: 520px; overflow: hidden; border-radius: 14px; background: #000000; }
.illustration-connexion img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center bottom; }

/* Page d'erreur sans compte */
.page-simple { max-width: 560px; margin: 0 auto; padding: 64px 16px; }

@media (max-width: 860px) {
  body.connexion { padding: 12px; }
  .cadre-connexion { grid-template-columns: 1fr; min-height: calc(100vh - 24px); }
  .illustration-connexion { display: none; }
  .colonne-connexion { padding: 40px 20px; }
}

@media (max-width: 760px) {
  .rangee { grid-template-columns: 1fr; }
  .grille-fiche { grid-template-columns: 1fr; gap: 2px; }
  .grille-fiche dd { margin-bottom: 10px; }
  .compte { margin-left: 0; width: 100%; justify-content: space-between; }
  .compte-nom { text-align: left; }
  th, td { padding: 8px 10px; }
}
"#;
