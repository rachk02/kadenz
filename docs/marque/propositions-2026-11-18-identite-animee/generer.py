"""Identité animée : cinq directions autour du logo Quatre pièces (demande de
l'utilisateur du 2026-11-18 : « plusieurs styles, avec motifs et autres et si
possible avec des animations »).

Le logo, ses couleurs et ses polices ne changent pas (charte docs/marque/README.md).
Chaque direction a une scène 16:9, une affiche 9:16 et un motif, tous animés
dans index.html (ouvrir dans Chrome ou Edge). Les textes viennent de
l'application (chaînes @tr) et de la charte ; aucun n'est inventé.

Usage : py -3.13 generer.py            (écrit index.html et planche-identite-animee.png)
Paramètres de la page : ?fige=1 fige chaque direction sur son moment clé ;
?seul=A&cadre=scene&t=1.5 n'affiche qu'un cadre à un instant donné (export vidéo).
"""
import base64
import re
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

IMAGES = {
    "logoH": "kadenz-logo-horizontal.svg",
    "logoHN": "kadenz-logo-horizontal-negatif.svg",
    "logoE": "kadenz-logo-empile.svg",
    "logoEN": "kadenz-logo-empile-negatif.svg",
    "mot": "kadenz-mot.svg",
    "motN": "kadenz-mot-negatif.svg",
}

DIRECTIONS = [
    {
        "id": "A", "nom": "Trame",
        "idee": "Les pièces se composent de points, comme une impression en trame. "
                "C'est la suite directe des nouveaux écrans de connexion et de présentation.",
        "couleurs": [("Nuit", "#1A2259", "fond"), ("Indigo", "#2D3A8C", "champ de points"),
                     ("Ambre", "#E3A23B", "pièces"), ("#C5CAE0", "#C5CAE0", "texte secondaire")],
        "mouvement": ["Les quatre pièces apparaissent l'une après l'autre, leurs points grossissent.",
                      "Une onde de lumière traverse la trame en continu.",
                      "Boucle de 8 s ; affiche : le disque seul, en grand."],
        "usages": "Écran de démarrage, vidéos, affiches, réseaux sociaux.",
        "ecart": "Pièces en Ambre sur la Nuit, déjà pratiqué sur les écrans refaits.",
    },
    {
        "id": "B", "nom": "Cadence",
        "idee": "Kadenz, c'est la cadence : les quatre pièces entrent sur quatre temps, "
                "puis les trois temps (blanc, blanc, Ambre) marquent la mesure.",
        "couleurs": [("Indigo", "#2D3A8C", "fond"), ("Blanc", "#FFFFFF", "pièces, mot"),
                     ("Ambre", "#E3A23B", "temps fort"), ("Nuit", "#1A2259", "affiche, motif")],
        "mouvement": ["Un temps toutes les 0,5 s ; chaque pièce entre avec un léger rebond.",
                      "Un cercle ton sur ton part du symbole à chaque temps.",
                      "Boucle de 7 s ; affiche : les quatre familles de modules s'allument tour à tour."],
        "usages": "Ouverture de vidéo, animation du logo, salon, écran d'attente.",
        "ecart": "Disque en Ambre pendant l'animation (le symbole reste d'une seule couleur dans la charte).",
    },
    {
        "id": "C", "nom": "Tournant",
        "idee": "Le motif Tournant se met en mouvement : chaque quart de disque tourne d'un quart "
                "de tour au passage d'une onde. Le logo se pose sur une zone calme où les pièces s'effacent.",
        "couleurs": [("Nuit", "#1A2259", "fond"), ("Pièces du motif", "#232C6B", "quarts de disque"),
                     ("Blanc", "#FFFFFF", "logo"), ("Ambre", "#E3A23B", "troisième temps")],
        "mouvement": ["Quatre ondes en diagonale par boucle ; chaque passage fait tourner les pièces d'un quart de tour.",
                      "La zone calme reste fixe, le logo ne bouge pas.",
                      "Boucle de 6 s, sans raccord visible."],
        "usages": "Grands fonds, stand, totem numérique, couverture de vidéo.",
        "ecart": "Aucun : motif ton sur ton, conforme à la charte.",
    },
    {
        "id": "D", "nom": "Épure",
        "idee": "La construction du symbole devient le décor : canevas de 256, marge de 16, jeu de 16, "
                "cases de 104. Un style de plan, précis et clair, sur fond Brume.",
        "couleurs": [("Brume", "#F4F5F9", "fond"), ("Indigo", "#2D3A8C", "tracé, symbole"),
                     ("Trait", "#D9DCE3", "grille"), ("Encre", "#1F2933", "texte")],
        "mouvement": ["La grille de construction se trace, puis les cotes apparaissent.",
                      "Le contour de chaque pièce se dessine, puis le symbole se remplit.",
                      "Boucle de 9 s ; motif : les pièces se tracent en vague."],
        "usages": "Documentation, manuels, site, présentation technique.",
        "ecart": "Aucun : couleurs et construction de la charte.",
    },
    {
        "id": "E", "nom": "Flux",
        "idee": "Les documents du logiciel portent l'identité : du devis à l'avoir, chaque feuille "
                "arrive à son tour, marquée d'une pièce, et la facture reçoit l'état Payé.",
        "couleurs": [("Brume", "#F4F5F9", "fond"), ("Blanc", "#FFFFFF", "feuilles"),
                     ("Indigo", "#2D3A8C", "marques"), ("Succès", "#067647", "état Payé")],
        "mouvement": ["Les cinq documents glissent l'un après l'autre (0,5 s d'écart).",
                      "Le tampon Payé se pose sur la facture, puis le titre apparaît.",
                      "Boucle de 9 s ; motif : des feuilles qui défilent."],
        "usages": "Présentation commerciale, site, vidéo de démonstration.",
        "ecart": "Couleur d'état Succès utilisée dans une image de marque.",
    },
]


def donnees_image(fichier):
    texte = (MARQUE / fichier).read_text(encoding="utf-8")
    vb = re.search(r'viewBox="([^"]+)"', texte).group(1).split()
    ratio = float(vb[2]) / float(vb[3])
    uri = "data:image/svg+xml;base64," + base64.b64encode(texte.encode("utf-8")).decode("ascii")
    return uri, ratio


def section(d):
    puces = "".join(f'<span class="puce"><i style="background:{hexa}"></i><b>{nom}</b><small>{role}</small></span>'
                    for nom, hexa, role in d["couleurs"])
    mouv = "".join(f"<li>{m}</li>" for m in d["mouvement"])
    return f"""<section id="dir-{d['id']}">
  <header><span class="lettre">{d['id']}</span><h2>{d['nom']}</h2><p>{d['idee']}</p></header>
  <div class="grille">
    <canvas data-dir="{d['id']}" data-cadre="scene" width="1280" height="720"></canvas>
    <canvas data-dir="{d['id']}" data-cadre="affiche" width="540" height="960"></canvas>
  </div>
  <div class="infos">
    <canvas data-dir="{d['id']}" data-cadre="motif" width="480" height="480"></canvas>
    <div><h3>Couleurs</h3><div class="puces">{puces}</div></div>
    <div><h3>Mouvement</h3><ul>{mouv}</ul></div>
    <div><h3>Pour</h3><p>{d['usages']}</p><h3>Écart à la charte</h3><p>{d['ecart']}</p></div>
  </div>
</section>"""


SCRIPT = r"""
const IMG_SRC = %%IMAGES%%;
const RATIO = %%RATIOS%%;
const PI = Math.PI;
const PIECES = {
  carre: new Path2D("M16 16 L120 16 L120 120 L16 120 Z"),
  quart: new Path2D("M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z"),
  tri: new Path2D("M16 136 L120 240 L16 240 Z"),
  disque: new Path2D("M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z"),
};
const ORDRE = ["carre", "quart", "tri", "disque"];
const CENTRE = { carre: [68, 68], quart: [188, 68], tri: [68, 188], disque: [188, 188] };
const PERIM = { carre: 416, quart: 377, tri: 356, disque: 337 };
const C = { nuit: "#1A2259", indigo: "#2D3A8C", motif: "#232C6B", ambre: "#E3A23B", brume: "#F4F5F9",
  encre: "#1F2933", ardoise: "#52606D", trait: "#D9DCE3", pale: "#E8EAF6", second: "#C5CAE0",
  succes: "#067647", blanc: "#FFFFFF", motifClair: "#E6E9F0" };

const clamp = (x, a = 0, b = 1) => Math.max(a, Math.min(b, x));
const lin = (t, a, d) => clamp((t - a) / d);
const eOut = x => 1 - Math.pow(1 - x, 3);
const eInOut = x => x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
const eBack = x => { const c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2); };
const hash = (i, j) => (((i * 73856093) ^ (j * 19349663)) >>> 0) % 4;

function dans(k, u, v) {
  switch (k) {
    case "carre": return u >= 16 && u <= 120 && v >= 16 && v <= 120;
    case "quart": return u >= 136 && v <= 120 && (u - 136) ** 2 + (v - 120) ** 2 <= 105.56 ** 2;
    case "tri": return u >= 16 && v <= 240 && v >= 136 && (u - 16) <= (v - 136);
    case "disque": return (u - 188) ** 2 + (v - 188) ** 2 <= 53.56 ** 2;
  }
  return false;
}

const IMG = {};
function image(c, nom, x, y, h, alpha = 1) {
  const w = h * RATIO[nom];
  if (alpha > 0 && IMG[nom]) { c.save(); c.globalAlpha *= alpha; c.drawImage(IMG[nom], x, y, w, h); c.restore(); }
  return w;
}
function police(o) { return `${o.poids || 400} ${o.taille || 20}px ${o.titre ? '"Space Grotesk"' : "Inter"}, sans-serif`; }
function texte(c, s, x, y, o = {}) {
  if ((o.alpha ?? 1) <= 0) return;
  c.save(); c.globalAlpha *= (o.alpha ?? 1); c.fillStyle = o.couleur || C.encre; c.font = police(o);
  c.textAlign = o.align || "left"; c.textBaseline = "alphabetic"; c.fillText(s, x, y); c.restore();
}
function paragraphe(c, s, x, y, largeur, interligne, o = {}) {
  c.save(); c.font = police(o);
  let ligne = "", yy = y;
  for (const m of s.split(" ")) {
    const essai = ligne ? ligne + " " + m : m;
    if (c.measureText(essai).width > largeur && ligne) { texte(c, ligne, x, yy, o); ligne = m; yy += interligne; }
    else ligne = essai;
  }
  if (ligne) texte(c, ligne, x, yy, o);
  c.restore(); return yy;
}
// Une pièce seule, centrée sur (cx, cy), côté de case s.
function unePiece(c, k, cx, cy, s, couleur, echelle = 1) {
  if (echelle <= 0.001) return;
  const [px, py] = CENTRE[k];
  c.save(); c.translate(cx, cy); c.scale(s / 104 * echelle, s / 104 * echelle); c.translate(-px, -py);
  c.fillStyle = couleur; c.fill(PIECES[k]); c.restore();
}
function symbole(c, x, y, s, couleurs, echelles) {
  c.save(); c.translate(x, y); c.scale(s / 256, s / 256);
  ORDRE.forEach((k, i) => {
    const e = echelles ? echelles[i] : 1; if (e <= 0.001) return;
    const [cx, cy] = CENTRE[k];
    c.save(); c.translate(cx, cy); c.scale(e, e); c.translate(-cx, -cy);
    c.fillStyle = Array.isArray(couleurs) ? couleurs[i] : couleurs; c.fill(PIECES[k]); c.restore();
  });
  c.restore();
}
function troisTemps(c, x, y, l, ep, jeu, prog, alpha = 1) {
  const cols = [C.blanc, C.blanc, C.ambre];
  c.save(); c.globalAlpha *= alpha;
  for (let i = 0; i < 3; i++) { const p = prog ? prog[i] : 1; if (p > 0) { c.fillStyle = cols[i]; c.fillRect(x + i * (l + jeu), y, l * p, ep); } }
  c.restore();
}
function fond(c, w, h, couleur) { c.fillStyle = couleur; c.fillRect(0, 0, w, h); }

// Trame : points ambre dans les pièces, champ de points indigo ailleurs.
function trame(c, w, h, o) {
  const { ox, oy, S, pas, liste, croissance, T } = o;
  for (let y = pas / 2; y < h; y += pas) for (let x = pas / 2; x < w; x += pas) {
    const u = (x - ox) / S * 256, v = (y - oy) / S * 256;
    const onde = 0.5 + 0.5 * Math.sin((x * 0.9 + y * 0.6) * 0.011 - T * 2.4);
    let k = null; for (const p of liste) if (dans(p, u, v)) { k = p; break; }
    if (k) {
      const g = croissance[ORDRE.indexOf(k)];
      if (g > 0.01) {
        const d = (u + v) / 512, lum = Math.min(1, Math.max(0.42, 0.3 + d * 1.2));
        c.fillStyle = C.ambre; c.beginPath(); c.arc(x, y, pas * 0.5 * lum * g * (0.86 + 0.14 * onde), 0, 2 * PI); c.fill();
        continue;
      }
    }
    c.fillStyle = o.champ || C.indigo; c.beginPath(); c.arc(x, y, pas * (0.09 + 0.07 * onde), 0, 2 * PI); c.fill();
  }
}

// Tournant : quarts de disque qui tournent d'un quart de tour au passage d'une onde.
function tournant(c, w, h, T, cell, calme) {
  fond(c, w, h, C.nuit);
  const cols = Math.ceil(w / cell), rows = Math.ceil(h / cell), m = cell / 2, R = cell * 0.96;
  c.fillStyle = C.motif;
  for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
    const cx = i * cell + m, cy = j * cell + m;
    const f = calme ? calme(cx, cy) : 1; if (f <= 0.02) continue;
    const phase = T / 1.5 - (i + j) * 0.06, n = Math.floor(phase), fr = phase - n;
    const rot = (hash(i, j) + n + eInOut(clamp(fr / 0.35))) * PI / 2;
    c.save(); c.translate(cx, cy); c.rotate(rot); c.scale(f, f);
    c.beginPath(); c.moveTo(-m, m); c.lineTo(-m, m - R); c.arc(-m, m, R, -PI / 2, 0); c.closePath(); c.fill();
    c.restore();
  }
}

// Épure : grille de construction, cotes, contours tracés, remplissage.
function construction(c, x0, y0, S, T, cotes = true) {
  const k = S / 256, vals = [0, 16, 120, 136, 240, 256];
  const efface = 1 - 0.6 * lin(T, 4.2, 0.8);
  c.save(); c.strokeStyle = "#C3C9D6"; c.lineWidth = 1; c.globalAlpha *= efface;
  vals.forEach((v, i) => {
    const p = eOut(lin(T, 0.1 + i * 0.08, 0.9)); if (p <= 0) return;
    const L = (S + 48) * p;
    c.beginPath(); c.moveTo(x0 + v * k, y0 - 24); c.lineTo(x0 + v * k, y0 - 24 + L); c.stroke();
    c.beginPath(); c.moveTo(x0 - 24, y0 + v * k); c.lineTo(x0 - 24 + L, y0 + v * k); c.stroke();
  });
  c.restore();
  if (cotes) {
    const a = lin(T, 1.1, 0.6) * efface;
    const segs = [[0, 16, "16"], [16, 120, "104"], [120, 136, "16"], [136, 240, "104"], [240, 256, "16"]];
    c.save(); c.globalAlpha *= a; c.strokeStyle = C.ardoise; c.lineWidth = 1;
    segs.forEach(([a1, b1, t]) => {
      const xa = x0 + a1 * k, xb = x0 + b1 * k, y = y0 - 40;
      c.beginPath(); c.moveTo(xa + 2, y); c.lineTo(xb - 2, y); c.moveTo(xa + 2, y - 4); c.lineTo(xa + 2, y + 4); c.moveTo(xb - 2, y - 4); c.lineTo(xb - 2, y + 4); c.stroke();
      texte(c, t, (xa + xb) / 2, y - 9, { taille: 15, couleur: C.ardoise, align: "center" });
    });
    const yb = y0 + S + 40;
    c.beginPath(); c.moveTo(x0, yb); c.lineTo(x0 + S, yb); c.moveTo(x0, yb - 4); c.lineTo(x0, yb + 4); c.moveTo(x0 + S, yb - 4); c.lineTo(x0 + S, yb + 4); c.stroke();
    texte(c, "256", x0 + S / 2, yb + 22, { taille: 15, couleur: C.ardoise, align: "center" });
    c.restore();
  }
  c.save(); c.translate(x0, y0); c.scale(k, k);
  ORDRE.forEach((p, i) => {
    const t = eInOut(lin(T, 1.8 + i * 0.45, 0.9)); if (t <= 0) return;
    c.setLineDash([PERIM[p] * t, 2000]); c.lineWidth = 2 / k; c.strokeStyle = C.indigo; c.stroke(PIECES[p]);
  });
  c.setLineDash([]);
  const remp = lin(T, 4.0, 0.8);
  if (remp > 0) { c.globalAlpha = remp; c.fillStyle = C.indigo; ORDRE.forEach(p => c.fill(PIECES[p])); }
  c.restore();
}

function feuille(c, x, y, w, h, nom, piece, alpha) {
  if (alpha <= 0) return;
  c.save(); c.globalAlpha *= alpha;
  c.fillStyle = "rgba(26,34,89,0.06)"; c.fillRect(x + 4, y + 6, w, h);
  c.fillStyle = C.blanc; c.fillRect(x, y, w, h); c.strokeStyle = C.trait; c.lineWidth = 1; c.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
  unePiece(c, piece, x + 16 + 11, y + 16 + 11, 22, C.indigo);
  paragraphe(c, nom, x + 16, y + 68, w - 32, 19, { taille: 15, poids: 600, titre: true, couleur: C.nuit });
  c.fillStyle = C.trait;
  const largeurs = [0.86, 0.62, 0.74, 0.5, 0.68];
  largeurs.forEach((l, i) => c.fillRect(x + 16, y + h * 0.46 + i * 14, (w - 32) * l, 5));
  c.fillStyle = C.pale; c.fillRect(x + w * 0.5, y + h - 40, w * 0.5 - 16, 18);
  c.restore();
}
function tampon(c, cx, cy, T, debut) {
  const a = lin(T, debut, 0.2); if (a <= 0) return;
  const s = 1.5 - 0.5 * eOut(lin(T, debut, 0.35));
  c.save(); c.globalAlpha *= a; c.translate(cx, cy); c.rotate(-0.17); c.scale(s, s);
  c.strokeStyle = C.succes; c.lineWidth = 2.5; c.strokeRect(-46, -20, 92, 40);
  texte(c, "Payé", 0, 9, { taille: 24, poids: 700, titre: true, couleur: C.succes, align: "center" });
  c.restore();
}

const DOCS = ["Devis", "Bon de commande", "Bordereau de livraison", "Facture", "Avoir"];

const DIR = {
  A: {
    periode: 8, moment: 4.2,
    scene(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const sortie = 1 - lin(T, 7.3, 0.7);
      const cr = ORDRE.map((k, i) => eOut(lin(T, 0.3 + i * 0.4, 0.8)) * sortie);
      trame(c, w, h, { ox: w * 0.5, oy: h * 0.12, S: h * 0.76, pas: 16, liste: ORDRE, croissance: cr, T });
      const a = lin(T, 2.0, 0.6) * sortie;
      image(c, "logoHN", 64, 56, 32, a);
      texte(c, "Gestion commerciale", 64, h - 150, { taille: 44, poids: 700, titre: true, couleur: C.blanc, alpha: a });
      paragraphe(c, "La facturation de votre entreprise, en FCFA et en sept langues.", 64, h - 104, 460, 30, { taille: 20, couleur: C.second, alpha: a });
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const sortie = 1 - lin(T, 7.3, 0.7), g = eOut(lin(T, 0.3, 1.2)) * sortie;
      const S = 190 / 53.56 * 256;
      trame(c, w, h, { ox: 270 - 188 / 256 * S, oy: 330 - 188 / 256 * S, S, pas: 14, liste: ["disque"], croissance: [0, 0, 0, g], T });
      const a = lin(T, 1.6, 0.6) * sortie;
      image(c, "logoHN", 48, 48, 28, a);
      texte(c, "Du devis à l'avoir", 48, 700, { taille: 50, poids: 700, titre: true, couleur: C.blanc, alpha: a });
      paragraphe(c, "Chaque document validé reçoit son numéro et s'exporte en PDF.", 48, 750, 420, 30, { taille: 21, couleur: C.second, alpha: a });
      troisTemps(c, 48, 880, 44, 6, 10, null, a);
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const pas = 16;
      for (let y = pas / 2; y < h; y += pas) for (let x = pas / 2; x < w; x += pas) {
        const d = Math.hypot(x - w * 0.3, y - h * 0.7);
        const o = 0.5 + 0.5 * Math.sin(d * 0.035 - T * 2.2);
        c.fillStyle = C.indigo; c.beginPath(); c.arc(x, y, pas * (0.08 + 0.3 * o * o), 0, 2 * PI); c.fill();
      }
    },
  },
  B: {
    periode: 7, moment: 4.6,
    scene(c, w, h, T) {
      fond(c, w, h, C.indigo);
      const b = 0.5, S = h * 0.36, cx = w / 2, cy = h * 0.38;
      const fin = 1 - lin(T, 6.0, 0.4);
      for (let k = 0; k < 8; k++) {
        const age = T - k * b; if (age < 0 || age > 1.4) continue;
        c.save(); c.globalAlpha = (1 - age / 1.4) * 0.5 * fin; c.strokeStyle = "#4351A8"; c.lineWidth = 2;
        c.beginPath(); c.arc(cx, cy, S * 0.72 + age * 420, 0, 2 * PI); c.stroke(); c.restore();
      }
      const ech = ORDRE.map((k, i) => eBack(lin(T, i * b, 0.35)) * (1 - eInOut(lin(T, 6.0 + (3 - i) * 0.1, 0.3))));
      symbole(c, cx - S / 2, cy - S / 2, S, [C.blanc, C.blanc, C.blanc, C.ambre], ech);
      const am = lin(T, 3.5, 0.4) * fin;
      const mw = 44 * RATIO.motN; image(c, "motN", cx - mw / 2, cy + S / 2 + 34, 44, am);
      troisTemps(c, cx - 110, cy + S / 2 + 106, 64, 8, 14, [0, 1, 2].map(i => eOut(lin(T, (4 + i) * b, 0.2))), fin);
      texte(c, "Gestion commerciale", cx, cy + S / 2 + 160, { taille: 22, couleur: C.second, align: "center", alpha: am });
      for (let i = 0; i < 4; i++) {
        const actif = Math.floor(T / b) % 4 === i;
        c.fillStyle = actif ? (i === 3 ? C.ambre : C.blanc) : "#4351A8"; c.fillRect(64 + i * 18, 64, 10, 10);
      }
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const b = 0.5, actif = Math.floor(T / b) % 4, f = T / b - Math.floor(T / b);
      texte(c, "Gestion commerciale", 48, 92, { taille: 34, poids: 700, titre: true, couleur: C.blanc });
      const noms = ["Ventes", "Commandes", "Livraisons", "Paiements"];
      ORDRE.forEach((k, i) => {
        const y = 200 + i * 160, on = i === actif;
        unePiece(c, k, 110, y + 50, 104, on ? C.ambre : "#2F3A86", on ? 0.92 + 0.08 * eBack(clamp(f / 0.6)) : 0.9);
        texte(c, noms[i], 200, y + 62, { taille: 34, poids: 600, titre: true, couleur: on ? C.blanc : "#9AA3C7" });
      });
      image(c, "logoHN", 48, 860, 28);
      troisTemps(c, 340, 872, 44, 6, 10);
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const b = 0.5, cell = 60, actif = Math.floor(T / b) % 8, f = T / b - Math.floor(T / b);
      for (let j = 0; j < 8; j++) for (let i = 0; i < 8; i++) {
        const k = ORDRE[(i + j) % 4], on = i === actif;
        const e = on ? 0.62 + 0.3 * eOut(clamp(f / 0.7)) : 0.92;
        unePiece(c, k, i * cell + cell / 2, j * cell + cell / 2, cell * 0.9, on && j === actif ? C.ambre : C.motif, e);
      }
    },
  },
  C: {
    periode: 6, moment: 2.0,
    scene(c, w, h, T) {
      const calme = (x, y) => clamp((Math.hypot((x - w * 0.25) / (w * 0.25), (y - h * 0.5) / (h * 0.42)) - 0.75) / 0.6);
      tournant(c, w, h, T, 80, calme);
      const lw = image(c, "logoHN", 0, -999, 46, 0);
      image(c, "logoHN", w * 0.25 - lw / 2, h * 0.5 - 52, 46);
      troisTemps(c, w * 0.25 - 77, h * 0.5 + 18, 44, 6, 10);
      texte(c, "Gestion commerciale", w * 0.25, h * 0.5 + 70, { taille: 22, couleur: C.second, align: "center" });
    },
    affiche(c, w, h, T) {
      const calme = (x, y) => clamp((Math.hypot((x - w * 0.5) / (w * 0.62), (y - h * 0.72) / (h * 0.22)) - 0.7) / 0.5);
      tournant(c, w, h, T, 68, calme);
      const ew = 150 * RATIO.logoEN; image(c, "logoEN", w / 2 - ew / 2, h * 0.72 - 110, 150);
      troisTemps(c, w / 2 - 77, h * 0.72 + 64, 44, 6, 10);
      texte(c, "Gestion commerciale", w / 2, h * 0.72 + 112, { taille: 24, couleur: C.second, align: "center" });
    },
    motif(c, w, h, T) { tournant(c, w, h, T, 60, null); },
  },
  D: {
    periode: 9, moment: 6.0,
    scene(c, w, h, T) {
      fond(c, w, h, C.brume);
      const a = 1 - lin(T, 8.4, 0.6);
      c.save(); c.globalAlpha = a;
      const S = h * 0.6; construction(c, 120, (h - S) / 2, S, T);
      image(c, "mot", w * 0.56, 200, 64, lin(T, 4.6, 0.6));
      paragraphe(c, "Rigoureux, régulier, clair, sobre, fiable.", w * 0.56, 340, 470, 38, { taille: 30, poids: 600, titre: true, couleur: C.encre, alpha: lin(T, 4.9, 0.6) });
      paragraphe(c, "Canevas de 256, marge de 16, jeu de 16, cases de 104.", w * 0.56, 440, 470, 26, { taille: 18, couleur: C.ardoise, alpha: lin(T, 5.2, 0.6) });
      c.restore();
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.blanc);
      const a = 1 - lin(T, 8.4, 0.6);
      c.save(); c.globalAlpha = a;
      construction(c, 100, 150, 340, T);
      image(c, "mot", 48, 640, 52, lin(T, 4.6, 0.6));
      texte(c, "Gestion commerciale", 48, 770, { taille: 36, poids: 700, titre: true, couleur: C.nuit, alpha: lin(T, 4.9, 0.6) });
      paragraphe(c, "Canevas de 256, marge de 16, jeu de 16, cases de 104.", 48, 815, 440, 26, { taille: 18, couleur: C.ardoise, alpha: lin(T, 5.2, 0.6) });
      c.restore();
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.brume);
      const cell = 80;
      for (let j = 0; j < 6; j++) for (let i = 0; i < 6; i++) {
        const k = ORDRE[(i + j) % 4], loc = ((T - (i + j) * 0.15) % 4.5 + 4.5) % 4.5;
        const p = eInOut(lin(loc, 0, 1.2)), a = 1 - lin(loc, 3.4, 0.8);
        if (p <= 0 || a <= 0) continue;
        const [px, py] = CENTRE[k], s = cell * 0.8 / 104;
        c.save(); c.globalAlpha = a; c.translate(i * cell + cell / 2, j * cell + cell / 2); c.scale(s, s); c.translate(-px, -py);
        c.fillStyle = C.motifClair; c.globalAlpha = a * lin(loc, 1.0, 0.6); c.fill(PIECES[k]);
        c.globalAlpha = a; c.setLineDash([PERIM[k] * p, 2000]); c.lineWidth = 1.5 / s; c.strokeStyle = C.indigo; c.stroke(PIECES[k]);
        c.restore();
      }
    },
  },
  E: {
    periode: 9, moment: 5.0,
    scene(c, w, h, T) {
      fond(c, w, h, C.brume);
      const fw = 160, fh = 240;
      DOCS.forEach((nom, i) => {
        const p = eOut(lin(T, 0.3 + i * 0.5, 0.7)), q = eInOut(lin(T, 8.2 + i * 0.06, 0.6));
        const x = 360 + i * 172 + (1 - p) * (fw + 200) - q * 60;
        feuille(c, x, 110, fw, fh, nom, ORDRE[i % 4], p * (1 - q));
        if (i === 3) { c.save(); c.globalAlpha = 1 - q; tampon(c, x + fw / 2, 110 + fh * 0.62, T, 3.2); c.restore(); }
      });
      const a = 1 - lin(T, 8.2, 0.6);
      image(c, "logoH", 64, 56, 32, a);
      texte(c, "Du devis à l'avoir", 64, 480, { taille: 46, poids: 700, titre: true, couleur: C.nuit, alpha: lin(T, 3.6, 0.6) * a });
      paragraphe(c, "Chaque document validé reçoit son numéro et s'exporte en PDF.", 64, 526, 620, 30, { taille: 21, couleur: C.ardoise, alpha: lin(T, 3.9, 0.6) * a });
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.brume);
      const fw = 270, fh = 380, a = 1 - lin(T, 8.2, 0.6);
      image(c, "logoH", 48, 44, 28, a);
      // Feuilles en escalier : la bande visible de chaque feuille montre son nom ;
      // le tampon Payé se pose à droite de la bande de la facture.
      DOCS.forEach((nom, i) => {
        const p = eOut(lin(T, 0.3 + i * 0.45, 0.7)), q = eInOut(lin(T, 8.2, 0.6));
        const x = 56 + i * 30, y = 110 + i * 80 + (1 - p) * 300;
        feuille(c, x, y, fw, fh, nom, ORDRE[i % 4], p * (1 - q));
        if (i === 3) { c.save(); c.globalAlpha = 1 - q; tampon(c, x + fw - 72, y + 46, T, 3.0); c.restore(); }
      });
      texte(c, "Du devis à l'avoir", 48, 846, { taille: 48, poids: 700, titre: true, couleur: C.nuit, alpha: lin(T, 3.4, 0.6) * a });
      paragraphe(c, "Chaque document validé reçoit son numéro et s'exporte en PDF.", 48, 890, 440, 30, { taille: 21, couleur: C.ardoise, alpha: lin(T, 3.7, 0.6) * a });
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.brume);
      const dx = 80, dy = 104, off = (T / 9 * 2 * dy) % dy;
      for (let j = -1; j < 6; j++) for (let i = 0; i < 6; i++) {
        const x = i * dx + 10, y = j * dy + 10 - off + (i % 2) * 52;
        c.fillStyle = C.blanc; c.fillRect(x, y, 60, 84); c.strokeStyle = C.trait; c.strokeRect(x + 0.5, y + 0.5, 59, 83);
        unePiece(c, ORDRE[(i + j + 4) % 4], x + 16, y + 16, 14, "#C9CEE6");
        c.fillStyle = C.motifClair; c.fillRect(x + 9, y + 40, 40, 4); c.fillRect(x + 9, y + 50, 30, 4);
      }
    },
  },
};

const params = new URLSearchParams(location.search);
const fige = params.get("fige") === "1";
const tFixe = params.has("t") ? parseFloat(params.get("t")) : null;
const seul = params.get("seul");
let pause = matchMedia("(prefers-reduced-motion: reduce)").matches;
let origine = performance.now(), decalage = 0;

const toiles = [...document.querySelectorAll("canvas[data-dir]")];
const visibles = new Set();
const obs = new IntersectionObserver(es => es.forEach(e => e.isIntersecting ? visibles.add(e.target) : visibles.delete(e.target)), { rootMargin: "200px" });
toiles.forEach(t => obs.observe(t));

if (seul) {
  document.body.classList.add("seul");
  const cadre = params.get("cadre") || "scene";
  toiles.forEach(t => { if (t.dataset.dir !== seul || t.dataset.cadre !== cadre) t.remove(); else document.body.replaceChildren(t); });
}

function dessiner(t) {
  for (const toile of document.querySelectorAll("canvas[data-dir]")) {
    if (!seul && !fige && tFixe === null && !visibles.has(toile)) continue;
    const d = DIR[toile.dataset.dir], c = toile.getContext("2d");
    const T = fige ? d.moment : (tFixe !== null ? tFixe : t % d.periode);
    c.setTransform(1, 0, 0, 1, 0, 0); c.globalAlpha = 1;
    d[toile.dataset.cadre](c, toile.width, toile.height, T);
  }
}
function boucle(now) {
  const t = pause ? decalage : decalage + (now - origine) / 1000;
  dessiner(t);
  if (!fige && tFixe === null) requestAnimationFrame(boucle);
  else document.body.dataset.pret = "1";
}
const bouton = document.getElementById("pause");
if (bouton) {
  const maj = () => bouton.textContent = pause ? "Lancer les animations" : "Mettre en pause";
  maj();
  bouton.onclick = () => {
    const now = performance.now();
    if (pause) origine = now; else decalage += (now - origine) / 1000;
    pause = !pause; maj();
  };
}

Promise.all([
  document.fonts.load('700 40px "Space Grotesk"'), document.fonts.load('600 20px "Space Grotesk"'),
  document.fonts.load("400 20px Inter"),
  ...Object.entries(IMG_SRC).map(([k, src]) => { const im = new Image(); im.src = src; IMG[k] = im; return im.decode(); }),
]).then(() => { origine = performance.now(); requestAnimationFrame(boucle); });
"""


def page():
    images, ratios = {}, {}
    for cle, fichier in IMAGES.items():
        images[cle], ratios[cle] = donnees_image(fichier)
    import json
    script = SCRIPT.replace("%%IMAGES%%", json.dumps(images)).replace("%%RATIOS%%", json.dumps(ratios))
    sections = "\n".join(section(d) for d in DIRECTIONS)
    sommaire = "".join(f'<a href="#dir-{d["id"]}"><b>{d["id"]}</b> {d["nom"]}</a>' for d in DIRECTIONS)
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Kadenz, identité animée</title>
<style>
@font-face {{ font-family:Inter; src:url(../planches/polices/Inter.woff2) format("woff2"); font-weight:100 900; }}
@font-face {{ font-family:"Space Grotesk"; src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype"); font-weight:300 700; }}
:root {{ --nuit:#1A2259; --indigo:#2D3A8C; --ambre:#E3A23B; --brume:#F4F5F9; --encre:#1F2933;
        --ardoise:#52606D; --trait:#D9DCE3; --blanc:#FFFFFF; }}
* {{ box-sizing:border-box; margin:0; }}
em, i {{ font-style:normal; }}
body {{ background:var(--brume); color:var(--encre); font-family:Inter, "Segoe UI", sans-serif;
        max-width:1680px; margin:0 auto; padding:56px 48px 80px; }}
body.seul {{ padding:0; max-width:none; background:#000; }}
body.seul canvas {{ display:block; width:auto; height:auto; }}
.entete {{ display:flex; flex-wrap:wrap; align-items:flex-end; justify-content:space-between; gap:24px;
           border-bottom:1px solid var(--trait); padding-bottom:28px; margin-bottom:28px; }}
h1 {{ font-family:"Space Grotesk", Inter, sans-serif; font-size:48px; color:var(--nuit); }}
.entete p {{ color:var(--ardoise); font-size:18px; max-width:760px; margin-top:10px; line-height:1.5; }}
#pause {{ font:600 15px Inter, sans-serif; background:var(--indigo); color:white; border:0; border-radius:4px;
          padding:12px 18px; cursor:pointer; }}
nav {{ display:flex; flex-wrap:wrap; gap:8px; margin-bottom:48px; }}
nav a {{ text-decoration:none; color:var(--nuit); background:white; border:1px solid var(--trait); border-radius:4px;
         padding:9px 14px; font-weight:500; }}
nav b {{ color:var(--indigo); margin-right:6px; }}
section {{ margin-bottom:72px; }}
section header {{ display:grid; grid-template-columns:auto 1fr; column-gap:18px; align-items:baseline; margin-bottom:20px; }}
.lettre {{ font-family:"Space Grotesk", Inter, sans-serif; font-weight:700; font-size:34px; color:var(--ambre); grid-row:span 2; }}
h2 {{ font-family:"Space Grotesk", Inter, sans-serif; font-size:34px; color:var(--nuit); }}
section header p {{ color:var(--ardoise); font-size:17px; line-height:1.5; max-width:980px; margin-top:6px; }}
.grille {{ display:grid; grid-template-columns:1fr 0.3164fr; gap:20px; align-items:start; }}
canvas {{ width:100%; height:auto; display:block; border:1px solid var(--trait); }}
.infos {{ display:grid; grid-template-columns:240px 1fr 1.3fr 1fr; gap:28px; margin-top:20px; align-items:start; }}
h3 {{ font-size:13px; letter-spacing:1.2px; text-transform:uppercase; color:var(--ardoise); margin-bottom:10px; font-weight:600; }}
.infos p, .infos li {{ font-size:15px; line-height:1.5; }}
.infos p + h3 {{ margin-top:18px; }}
.infos ul {{ padding-left:18px; }}
.puces {{ display:grid; gap:8px; }}
.puce {{ display:grid; grid-template-columns:28px 1fr; column-gap:10px; align-items:center; font-size:14px; }}
.puce i {{ width:28px; height:28px; border:1px solid var(--trait); grid-row:span 2; }}
.puce small {{ color:var(--ardoise); }}
footer {{ border-top:1px solid var(--trait); padding-top:24px; color:var(--ardoise); font-size:15px; line-height:1.6; }}
@media (max-width: 900px) {{
  body {{ padding:32px 16px 56px; }}
  .grille {{ grid-template-columns:1fr; }}
  .grille canvas[data-cadre="affiche"] {{ max-width:360px; }}
  .infos {{ grid-template-columns:1fr; }}
  .infos canvas {{ max-width:240px; }}
}}
</style></head>
<body>
<div class="entete">
  <div><h1>Identité animée</h1>
  <p>Cinq directions autour du logo Quatre pièces, qui ne change pas, ni ses couleurs ni ses polices.
  Chacune a son mouvement, son motif, une scène 16:9 et une affiche 9:16. On peut retenir une direction,
  ou en assembler plusieurs.</p></div>
  <button id="pause" type="button">Mettre en pause</button>
</div>
<nav>{sommaire}</nav>
{sections}
<footer>Textes repris de l'application (présentation, menus, documents, états de paiement) et de la charte
(ton, construction du symbole, familles de modules). Animations dessinées en direct dans la page ; une
vidéo de la direction retenue peut être exportée ensuite.</footer>
<script>{script}</script>
</body></html>
"""


if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "planche-identite-animee.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--disable-gpu",
                    "--window-size=1680,6200", f"--screenshot={png}", "--virtual-time-budget=6000",
                    "--allow-file-access-from-files", html.as_uri() + "?fige=1"], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(html)
    print(png)
