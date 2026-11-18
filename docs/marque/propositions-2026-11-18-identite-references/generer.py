"""Identité animée, deuxième tour : les trois directions retenues le 2026-11-18
(A Trame, A2 Trame de pièces, C Tournant), retravaillées d'après les références
transmises par l'utilisateur (quatre vidéos et quatre images, gardées hors du dépôt).

- A+ Anneaux : vidéos 1 à 3 (anneaux de points qui tournent lentement, petites sphères).
- A2+ Pièces et lignes : images 1 à 4 (formes géométriques pleines, faisceaux de lignes
  parallèles qui suivent les courbes, cercles en accent).
- C+ Carrousel : vidéo 4 (série d'affiches qui défile, deux couleurs fortes, grands chiffres).

Le logo, ses couleurs et ses polices ne changent pas. Les chiffres et les textes viennent de
l'application (langues, monnaie, documents, états de paiement, version) ; aucun n'est inventé.

Usage : py -3.13 generer.py   (Chrome requis ; réutilise la page de propositions-2026-11-18-identite-animee)
"""
import subprocess
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
sys.path.insert(0, str(ICI.parent / "propositions-2026-11-18-identite-animee"))
import generer as base  # noqa: E402

DIRECTIONS = [
    {
        "id": "A+", "nom": "Anneaux",
        "idee": "A Trame, d'après les vidéos 1 à 3 : des anneaux faits de milliers de points de tailles variées "
                "tournent très lentement autour d'un centre hors du cadre ; deux sphères suivent leur anneau. "
                "Version sombre sur la Nuit, version claire sur la Brume avec le disque à plat.",
        "couleurs": [("Nuit", "#1A2259", "fond sombre"), ("#C5CAE0", "#C5CAE0", "points sur la Nuit"),
                     ("Brume", "#F4F5F9", "fond clair"), ("Ambre", "#E3A23B", "points rares, disque, sphère")],
        "mouvement": ["Les anneaux tournent à deux vitesses, l'intérieur deux fois plus vite.",
                      "Les sphères avancent avec leur anneau ; rien ne clignote, le mouvement est continu.",
                      "Boucle de 12 s sans raccord (les points se répètent tous les quinze degrés)."],
        "usages": "Écran de démarrage, fonds de vidéo, affiches, écran d'attente.",
        "ecart": "Sphères éclairées (dégradé de lumière) et grain de points, nouveaux dans la charte.",
    },
    {
        "id": "A2+", "nom": "Pièces et lignes",
        "idee": "A2, d'après les images : chaque pièce se dessine pleine ou en faisceau de lignes parallèles qui "
                "suivent sa forme (carrés emboîtés, arcs, diagonales, cercles), avec des cercles en accent. "
                "Le symbole devient un signe construit, comme les caractères des affiches de référence.",
        "couleurs": [("Nuit", "#1A2259", "fond"), ("Brume", "#F4F5F9", "pièces pleines et lignes"),
                     ("Ambre", "#E3A23B", "lignes et cercles d'accent"), ("Pièces du motif", "#232C6B", "motif")],
        "mouvement": ["Les pièces se construisent l'une après l'autre : les pleines apparaissent, les lignes se tracent.",
                      "Sur l'affiche, les huit cases du signe se posent une à une.",
                      "Boucle de 8 s ; motif : les faisceaux se tracent et se retirent en vague."],
        "usages": "Affiches, couvertures de manuels, salon, réseaux sociaux.",
        "ecart": "Faisceaux de lignes dans les pièces, nouveau traitement à ajouter à la charte.",
    },
    {
        "id": "C+", "nom": "Carrousel",
        "idee": "C Tournant, d'après la vidéo 4 : une série de six affiches qui défile. Chacune porte le motif "
                "Tournant et un chiffre tiré du logiciel ; les fonds alternent Nuit, Ambre et Brume, et sur "
                "certaines les quarts de disque se dessinent en arcs parallèles.",
        "couleurs": [("Indigo", "#2D3A8C", "fond de la scène"), ("Nuit", "#1A2259", "affiches"),
                     ("Ambre", "#E3A23B", "affiches, arcs"), ("Brume", "#F4F5F9", "affiches")],
        "mouvement": ["Les affiches défilent de droite à gauche, une série complète en 24 s.",
                      "Dans chaque affiche, le motif Tournant continue de tourner.",
                      "Affiche seule : les six affiches se succèdent toutes les 4 s, en glissant vers le haut."],
        "usages": "Réseaux sociaux (série), écrans de salon, site, campagne de lancement.",
        "ecart": "Ambre en aplat de fond, avec texte Nuit (contraste 6,7:1).",
    },
]

AVANT = """<section class="refs">
  <h2>Ce que je retiens de tes références</h2>
  <div class="cartes">
    <div><h3>Vidéos 1 à 3, vers A+</h3><p>Grands anneaux de points fins, tailles et clartés variées, qui
    tournent lentement autour d'un centre hors du cadre ; quelques sphères éclairées. Fond presque noir,
    gris et une couleur chaude ; une version claire avec un grand disque à plat. Mouvement lent et continu.</p></div>
    <div><h3>Images 1 à 4, vers A2+</h3><p>Caractères construits avec des modules géométriques : traits
    pleins, faisceaux de lignes parallèles qui suivent les courbes, cercles en accent, mots courts à la
    verticale. Aplats sur fond noir, une ou deux couleurs par affiche.</p></div>
    <div><h3>Vidéo 4, vers C+</h3><p>Série d'affiches verticales qui défile : deux couleurs fortes qui
    alternent, grands chiffres, grilles de pixels, petites étiquettes. Ici les chiffres viennent du
    logiciel : 7 langues, FCFA, 6 documents, 3 états de paiement, version 1.0.0.</p></div>
  </div>
</section>
<style>
.refs { background:white; border:1px solid var(--trait); border-radius:4px; padding:28px 32px; margin-bottom:28px; }
.refs h2 { font-size:24px; margin-bottom:16px; }
.cartes { display:grid; grid-template-columns:repeat(3, 1fr); gap:28px; }
.cartes p { font-size:15px; line-height:1.55; }
@media (max-width: 900px) { .cartes { grid-template-columns:1fr; } }
</style>"""

FONCTIONS = r"""
function alea(graine) {
  let a = graine >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

// Anneaux de points : chaque bande [r0, r1, n, tours, clarté, part d'Ambre]. Les points sont
// dessinés une fois dans un calque par vitesse (tours), puis chaque calque tourne d'un bloc.
// Les points se répètent tous les 15 degrés et chaque calque tourne d'un ou deux
// vingt-quatrièmes de tour par période : la boucle ne montre aucun raccord.
const CACHE_ANNEAUX = new Map();
function anneaux(c, w, h, T, o) {
  let calques = CACHE_ANNEAUX.get(o.cle);
  if (!calques) {
    calques = new Map(); const rnd = alea(o.graine || 11), m = 24, densite = o.densite || 7;
    const R = Math.ceil(Math.max(...o.bandes.map(b => b[1]))) + 4;
    for (const [r0, r1, n, tours, clair, pa] of o.bandes) {
      if (!calques.has(tours)) {
        const toile = document.createElement("canvas"); toile.width = toile.height = 2 * R;
        calques.set(tours, { toile, R });
      }
      const g = calques.get(tours).toile.getContext("2d");
      for (let i = 0; i < n * densite; i++) {
        const r = r0 + (r1 - r0) * rnd(), th = rnd() * 2 * PI / m, s = 0.35 + 1.7 * Math.pow(rnd(), 2.8);
        const l = clair * (0.2 + 0.8 * rnd()), am = rnd() < pa;
        g.fillStyle = am ? C.ambre : o.couleur(l);
        for (let j = 0; j < m; j++) {
          const a = th + j * 2 * PI / m;
          g.beginPath(); g.arc(R + r * Math.cos(a), R + r * Math.sin(a), s * (o.taille || 1), 0, 2 * PI); g.fill();
        }
      }
    }
    CACHE_ANNEAUX.set(o.cle, calques);
  }
  const base = T / o.periode * 2 * PI / 24;
  c.save();
  if (o.zone) { c.beginPath(); c.rect(...o.zone); c.clip(); }
  for (const [tours, { toile, R }] of calques) {
    c.save(); c.translate(o.cx, o.cy); c.rotate(base * tours); c.drawImage(toile, -R, -R); c.restore();
  }
  c.restore();
}
function sphere(c, x, y, r, couleur, clair) {
  const g = c.createRadialGradient(x - r * 0.35, y - r * 0.35, r * 0.1, x, y, r);
  g.addColorStop(0, clair); g.addColorStop(1, couleur);
  c.fillStyle = g; c.beginPath(); c.arc(x, y, r, 0, 2 * PI); c.fill();
}
function orbite(c, cx, cy, r, a0, tours, T, periode, rayon, couleur, clair) {
  const a = a0 + T / periode * 2 * PI / 24 * tours;
  sphere(c, cx + r * Math.cos(a), cy + r * Math.sin(a), rayon, couleur, clair);
}

// Faisceau de lignes parallèles qui suit la forme de la pièce (unités du symbole).
function lignesPiece(c, k, prog, nb, couleur, ep) {
  c.save(); c.clip(PIECES[k]); c.strokeStyle = couleur; c.lineWidth = ep; c.lineCap = "butt";
  for (let i = 0; i < nb; i++) {
    const f = clamp(prog * nb - i); if (f <= 0) break;
    c.beginPath(); let L;
    if (k === "carre") {
      const e = (i + 0.5) * 52 / nb, a = 16 + e, b = 120 - e; L = 4 * (b - a);
      c.moveTo(a, a); c.lineTo(b, a); c.lineTo(b, b); c.lineTo(a, b); c.closePath();
    } else if (k === "quart") {
      const r = 105.56 * (i + 0.5) / nb; L = r * PI / 2; c.arc(136, 120, r, -PI / 2, 0);
    } else if (k === "tri") {
      const cc = -224 + (i + 0.5) * 104 / nb; L = Math.hypot(224 + cc, 224 + cc);
      c.moveTo(16, 16 - cc); c.lineTo(240 + cc, 240);
    } else {
      const r = 53.56 * (i + 0.5) / nb; L = 2 * PI * r; c.arc(188, 188, r, -PI / 2, 1.5 * PI);
    }
    c.setLineDash([L * f, L + 1]); c.stroke();
  }
  c.restore();
}
// Une case du signe : style 0 plein Brume, 1 lignes Brume, 2 lignes Ambre, 3 disque Ambre à anneau.
function caseSigne(c, k, style, cx, cy, s, p) {
  if (p <= 0) return;
  const [px, py] = CENTRE[k];
  c.save(); c.translate(cx, cy);
  const e = style === 0 || style === 3 ? eBack(clamp(p)) : 1;
  c.scale(s / 104 * e, s / 104 * e); c.translate(-px, -py);
  if (style === 0) { c.fillStyle = C.brume; c.fill(PIECES[k]); }
  else if (style === 1) lignesPiece(c, k, eInOut(clamp(p)), 6, C.brume, 5);
  else if (style === 2) lignesPiece(c, k, eInOut(clamp(p)), 7, C.ambre, 4);
  else {
    c.fillStyle = C.ambre; c.fill(PIECES[k]);
    const [qx, qy] = CENTRE[k];
    c.fillStyle = C.nuit; c.beginPath(); c.arc(qx, qy, 30, 0, 2 * PI); c.fill();
    c.fillStyle = C.brume; c.beginPath(); c.arc(qx, qy, 12, 0, 2 * PI); c.fill();
  }
  c.restore();
}
function texteVertical(c, s, x, y, o) {
  c.save(); c.translate(x, y); c.rotate(-PI / 2); c.letterSpacing = (o.espace || 0) + "px";
  texte(c, s, 0, 0, o); c.restore();
}

// Motif Tournant dans un rectangle ; calme(x, y) renvoie la taille relative (0 : case vide).
function tournantZone(c, x0, y0, w, h, T, cell, fondC, pieceC, calme, lignes) {
  c.save(); c.beginPath(); c.rect(x0, y0, w, h); c.clip();
  c.fillStyle = fondC; c.fillRect(x0, y0, w, h);
  const cols = Math.ceil(w / cell), rows = Math.ceil(h / cell), m = cell / 2, R = cell * 0.96;
  for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
    const lx = i * cell + m, ly = j * cell + m, f = calme ? calme(lx, ly) : 1;
    if (f <= 0.02) continue;
    const phase = T / 1.5 - (i + j) * 0.06, n = Math.floor(phase), fr = phase - n;
    const rot = (hash(i, j) + n + eInOut(clamp(fr / 0.35))) * PI / 2;
    c.save(); c.translate(x0 + lx, y0 + ly); c.rotate(rot); c.scale(f, f);
    if (lignes) {
      c.strokeStyle = pieceC; c.lineWidth = cell * 0.055;
      for (let q = 1; q <= 5; q++) { c.beginPath(); c.arc(-m, m, R * q / 5.3, -PI / 2, 0); c.stroke(); }
    } else {
      c.fillStyle = pieceC; c.beginPath(); c.moveTo(-m, m); c.lineTo(-m, m - R); c.arc(-m, m, R, -PI / 2, 0); c.closePath(); c.fill();
    }
    c.restore();
  }
  c.restore();
}

const AMBRE_FONCE = "#D29434";
// Les six affiches de la série, dessinées dans un cadre de 540 x 960.
function afficheSerie(c, idx, T) {
  const nette = (y0) => (x, y) => y < y0 ? 0 : 1;
  const entete = (sombre, num, clair) => {
    if (sombre === "nuit") image(c, "motN", 40, 40, 26);
    else if (sombre === "brume") image(c, "mot", 40, 40, 26);
    texte(c, num, 500, 62, { taille: 18, poids: 600, couleur: clair, align: "right" });
  };
  if (idx === 0) {
    tournantZone(c, 0, 0, 540, 960, T, 90, C.nuit, C.motif, (x, y) => (y < 110 || y > 560) ? 0 : 1);
    entete("nuit", "01", C.second);
    texte(c, "7", 32, 860, { taille: 300, poids: 700, titre: true, couleur: C.brume });
    texte(c, "langues", 230, 740, { taille: 46, poids: 700, titre: true, couleur: C.brume });
    paragraphe(c, "Français, English, Español, Türkçe, 日本語, 中文, العربية", 230, 790, 280, 28, { taille: 19, couleur: C.second });
  } else if (idx === 1) {
    c.fillStyle = C.ambre; c.fillRect(0, 0, 540, 960);
    tournantZone(c, 0, 520, 540, 440, T, 90, C.ambre, AMBRE_FONCE, null);
    entete("ambre", "02", C.nuit);
    texte(c, "FCFA", 34, 330, { taille: 150, poids: 700, titre: true, couleur: C.nuit });
    paragraphe(c, "La facturation de votre entreprise, en FCFA et en sept langues.", 40, 400, 440, 32, { taille: 23, couleur: C.nuit });
  } else if (idx === 2) {
    c.fillStyle = C.brume; c.fillRect(0, 0, 540, 960);
    tournantZone(c, 270, 110, 270, 400, T, 90, C.brume, C.indigo, null, true);
    entete("brume", "03", C.ardoise);
    texte(c, "6", 32, 480, { taille: 300, poids: 700, titre: true, couleur: C.indigo });
    texte(c, "documents", 40, 560, { taille: 46, poids: 700, titre: true, couleur: C.nuit });
    ["Devis", "Proforma", "Bon de commande", "Bordereau de livraison", "Facture", "Avoir"]
      .forEach((d, i) => texte(c, d, 40, 615 + i * 36, { taille: 23, couleur: C.encre }));
  } else if (idx === 3) {
    tournantZone(c, 0, 0, 540, 960, T, 90, C.nuit, C.ambre, (x, y) => y < 580 ? 0 : 1, true);
    entete("nuit", "04", C.second);
    texte(c, "3", 32, 400, { taille: 300, poids: 700, titre: true, couleur: C.brume });
    texte(c, "états de paiement", 40, 470, { taille: 42, poids: 700, titre: true, couleur: C.brume });
    texte(c, "Impayé, Partiel, Payé", 40, 515, { taille: 24, couleur: C.second });
  } else if (idx === 4) {
    const calme = (x, y) => clamp((Math.hypot((x - 270) / 250, (y - 500) / 230) - 0.8) / 0.4);
    tournantZone(c, 0, 0, 540, 960, T, 90, C.brume, C.motifClair, (x, y) => y < 110 ? 0 : calme(x, y));
    entete("aucun", "05", C.ardoise);
    const ew = 190 * RATIO.logoE; image(c, "logoE", 270 - ew / 2, 380, 190);
    texte(c, "Gestion commerciale", 270, 640, { taille: 26, poids: 600, couleur: C.nuit, align: "center" });
  } else {
    c.fillStyle = C.ambre; c.fillRect(0, 0, 540, 960);
    tournantZone(c, 0, 110, 540, 300, T, 90, C.ambre, AMBRE_FONCE, null, true);
    entete("ambre", "06", C.nuit);
    texte(c, "Version", 40, 520, { taille: 34, poids: 600, couleur: C.nuit });
    texte(c, "1.0.0", 30, 680, { taille: 160, poids: 700, titre: true, couleur: C.nuit });
    texte(c, "Gestion commerciale", 40, 760, { taille: 26, couleur: C.nuit });
  }
}
function afficheDans(c, idx, x, y, w, T) {
  const k = w / 540;
  c.save(); c.translate(x, y); c.scale(k, k);
  c.beginPath(); c.rect(0, 0, 540, 960); c.clip();
  afficheSerie(c, idx, T); c.restore();
}
"""

ENTREES = r"""
  "A+": {
    periode: 12, moment: 3.0,
    scene(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const cx = -0.28 * w, cy = 0.5 * h;
      anneaux(c, w, h, T, { cle: "A+s", cx, cy, periode: 12, couleur: l => `rgba(197,202,224,${l})`,
        bandes: [[0.50 * w, 0.58 * w, 300, 2, 0.5, 0], [0.60 * w, 0.615 * w, 110, 2, 0.95, 0.02],
                 [0.64 * w, 0.80 * w, 560, 1, 0.55, 0.025], [0.83 * w, 0.86 * w, 150, 1, 0.9, 0],
                 [0.89 * w, 0.93 * w, 170, 1, 0.45, 0.03]] });
      orbite(c, cx, cy, 0.6075 * w, -0.5, 2, T, 12, 12, "#8F98C2", "#FFFFFF");
      orbite(c, cx, cy, 0.72 * w, -0.08, 1, T, 12, 6, "#8F98C2", "#FFFFFF");
      orbite(c, cx, cy, 0.845 * w, 0.3, 1, T, 12, 26, "#B57718", "#F7C873");
      const x0 = 0.71 * w;
      image(c, "logoHN", x0, 250, 34);
      troisTemps(c, x0, 312, 44, 6, 10);
      texte(c, "Gestion commerciale", x0, 380, { taille: 34, poids: 700, titre: true, couleur: C.blanc });
      paragraphe(c, "La facturation de votre entreprise, en FCFA et en sept langues.", x0, 420, 330, 28, { taille: 19, couleur: C.second });
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.brume);
      const cx = -0.55 * w, cy = 0.36 * h, zone = [0, 0, w, 0.7 * h];
      anneaux(c, w, h, T, { cle: "A+a", cx, cy, periode: 12, zone, couleur: l => `rgba(45,58,140,${l})`,
        bandes: [[0.75 * w, 0.95 * w, 300, 2, 0.5, 0.03], [1.0 * w, 1.04 * w, 110, 2, 0.85, 0],
                 [1.1 * w, 1.45 * w, 480, 1, 0.4, 0.04], [1.5 * w, 1.56 * w, 120, 1, 0.75, 0]] });
      c.fillStyle = C.ambre; c.beginPath(); c.arc(0.36 * w, 0.3 * h, 0.2 * w, 0, 2 * PI); c.fill();
      texte(c, "Du devis à l'avoir", 48, 770, { taille: 48, poids: 700, titre: true, couleur: C.nuit });
      paragraphe(c, "Chaque document validé reçoit son numéro et s'exporte en PDF.", 48, 815, 440, 30, { taille: 21, couleur: C.ardoise });
      image(c, "logoH", 48, 895, 26);
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.nuit);
      anneaux(c, w, h, T, { cle: "A+m", cx: -0.25 * w, cy: -0.25 * h, periode: 12, couleur: l => `rgba(197,202,224,${l})`,
        bandes: [[0.5 * w, 0.62 * w, 240, 2, 0.5, 0.02], [0.66 * w, 0.69 * w, 90, 2, 0.9, 0],
                 [0.74 * w, 1.0 * w, 420, 1, 0.5, 0.03], [1.05 * w, 1.1 * w, 120, 1, 0.85, 0]] });
      orbite(c, -0.25 * w, -0.25 * h, 0.675 * w, 0.75, 2, T, 12, 9, "#8F98C2", "#FFFFFF");
    },
  },
  "A2+": {
    periode: 8, moment: 5.0,
    scene(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const sortie = 1 - lin(T, 7.3, 0.7), S = h * 0.78, x0 = w * 0.52, y0 = (h - S) / 2, s = S * 104 / 256;
      const styles = [1, 0, 2, 3];
      c.save(); c.globalAlpha = sortie;
      ORDRE.forEach((k, i) => {
        const [px, py] = CENTRE[k];
        caseSigne(c, k, styles[i], x0 + px / 256 * S, y0 + py / 256 * S, s, lin(T, 0.3 + i * 0.6, 0.9));
      });
      const a = lin(T, 2.8, 0.6);
      texteVertical(c, "GESTION COMMERCIALE", x0 - 26, y0 + S, { taille: 26, poids: 700, titre: true, couleur: C.ambre, alpha: a, espace: 3 });
      image(c, "logoHN", 64, 56, 32, a);
      paragraphe(c, "La facturation de votre entreprise, en FCFA et en sept langues.", 64, h - 120, 420, 30, { taille: 20, couleur: C.second, alpha: a });
      c.restore();
    },
    affiche(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const sortie = 1 - lin(T, 7.3, 0.7), cell = 170, x0 = 160, y0 = 100;
      const signe = [["carre", 1], ["quart", 0], ["tri", 0], ["disque", 3], ["quart", 2], ["carre", 0], ["disque", 0], ["tri", 1]];
      c.save(); c.globalAlpha = sortie;
      signe.forEach(([k, st], i) => {
        const col = i % 2, row = Math.floor(i / 2);
        caseSigne(c, k, st, x0 + col * (cell + 8) + cell / 2, y0 + row * (cell + 8) + cell / 2, cell, lin(T, 0.3 + i * 0.35, 0.8));
      });
      const a = lin(T, 3.4, 0.6);
      texteVertical(c, "GESTION COMMERCIALE", 132, y0 + 4 * (cell + 8) - 8, { taille: 28, poids: 700, titre: true, couleur: C.ambre, alpha: a, espace: 4 });
      image(c, "logoHN", 160, 860, 28, a);
      troisTemps(c, 370, 872, 44, 6, 10, null, a);
      c.restore();
    },
    motif(c, w, h, T) {
      fond(c, w, h, C.nuit);
      const cell = 80, n = Math.floor(T / 0.5) % 6;
      for (let j = 0; j < 6; j++) for (let i = 0; i < 6; i++) {
        const k = ORDRE[(i + j) % 4], loc = ((T - (i + j) * 0.12) % 8 + 8) % 8;
        const p = loc < 6 ? eInOut(clamp(loc / 1.2)) : 1 - eInOut(clamp((loc - 6) / 1.2));
        const [px, py] = CENTRE[k];
        c.save(); c.translate(i * cell + cell / 2, j * cell + cell / 2); c.scale(cell * 0.86 / 104, cell * 0.86 / 104); c.translate(-px, -py);
        if (i === n && j === n) { c.fillStyle = C.ambre; c.fill(PIECES[k]); }
        else lignesPiece(c, k, p, 5, "#2E3980", 6);
        c.restore();
      }
    },
  },
  "C+": {
    periode: 24, moment: 2.0,
    scene(c, w, h, T) {
      fond(c, w, h, C.indigo);
      const ph = 0.84 * h, pw = ph * 9 / 16, pas = pw + 28, cycle = 6 * pas;
      const dec = (T / 24 * cycle) % cycle, y = (h - ph) / 2;
      for (let i = -1; i < 6 + Math.ceil(w / pas); i++) {
        const x = 40 + i * pas - dec; if (x > w || x + pw < 0) continue;
        afficheDans(c, ((i % 6) + 6) % 6, x, y, pw, T);
      }
    },
    affiche(c, w, h, T) {
      const idx = Math.floor(T / 4) % 6, f = T % 4, off = eInOut(clamp((f - 3.5) / 0.5)) * h;
      afficheDans(c, idx, 0, -off, w, T);
      if (off > 0) afficheDans(c, (idx + 1) % 6, 0, h - off, w, T);
    },
    motif(c, w, h, T) { tournantZone(c, 0, 0, w, h, T, 60, C.nuit, "#2E3980", null, true); },
  },
"""

INTRO = ("Les trois directions retenues (A Trame, A2 Trame de pièces, C Tournant), retravaillées d'après tes "
         "quatre vidéos et tes quatre images. Le logo, ses couleurs et ses polices ne changent pas. Les versions "
         "du premier tour restent dans propositions-2026-11-18-identite-animee pour comparer.")
PIED = ("Références de l'utilisateur gardées hors du dépôt (C:\\Users\\rachk\\Kadenz-videos). Chiffres et textes "
        "tirés de l'application : langues de l'interface, monnaie, menus des documents, états de paiement, "
        "version 1.0.0. Les vidéos de référence n'ont pas de son.")

if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(base.page(DIRECTIONS, "Identité animée, deuxième tour", INTRO, PIED, AVANT, FONCTIONS, ENTREES),
                    encoding="utf-8")
    png = ICI / "planche-identite-references.png"
    subprocess.run([base.CHROME, "--headless=new", "--hide-scrollbars", "--disable-gpu",
                    "--window-size=1680,4600", f"--screenshot={png}", "--virtual-time-budget=6000",
                    "--allow-file-access-from-files", html.as_uri() + "?fige=1"], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(html)
    print(png)
