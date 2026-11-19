// Portail des licences : grandes colonnes corinthiennes en points, sur fond
// noir (proposition du 2026-11-19, kadenz-e6, d'après une référence de
// l'utilisateur). Trois colonnes en ruine, dessinées en niveaux de gris puis
// tramées : un point par case de la grille, gardé selon la clarté du dessin.
// Colonnes fixes (choix de l'utilisateur du 2026-11-19) : dessinées une
// fois, et de nouveau quand le panneau change de taille. L'ancienne réaction
// à la souris reste disponible par ?souris-active.
//
// Paramètres de l'adresse : ?capture (sans réglage de maquette),
// ?souris-active (les points s'écartent au passage de la souris).

(function () {
  "use strict";

  const toile = document.getElementById("piliers");
  if (!toile || !toile.getContext) {
    return;
  }
  const ctx = toile.getContext("2d");
  const panneau = toile.parentElement;
  const parametres = new URLSearchParams(window.location.search);
  const capture = parametres.has("capture");
  const reduit = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const reactif = parametres.has("souris-active");
  if (capture) {
    document.body.classList.add("capture");
  }

  const PAS = 4;
  const TAILLE = 1.7;
  const TEINTES = ["rgba(214, 218, 208, 0.42)", "rgba(214, 218, 208, 0.68)", "rgba(232, 235, 228, 0.95)"];
  const RESSORT = 0.0075;
  const AMORTI = 0.9;
  const PORTEE = 100;
  const POUSSEE = 1.8;

  let largeur = 0;
  let hauteur = 0;
  let ratio = 1;
  let points = [];
  const souris = { x: -1e5, y: -1e5 };

  // Hasard reproductible : les colonnes restent les mêmes d'une ouverture
  // à l'autre.
  let graine = 7;
  function hasard() {
    graine = (graine * 1664525 + 1013904223) % 4294967296;
    return graine / 4294967296;
  }

  // ----- Dessin d'une colonne, en clarté (blanc plus ou moins opaque) -----

  function colonne(c, cx, l, haut, bas, ruine) {
    const H = bas - haut;
    const hChapiteau = Math.min(H * 0.19, l * 1.55);
    const hBase = Math.min(H * 0.07, l * 0.6);
    const hautFut = haut + hChapiteau;
    const basFut = bas - hBase;
    const lBas = l;
    const lHaut = l * 0.86;

    // Fût : fond sombre, ombre à droite, arêtes et cannelures claires.
    c.globalAlpha = 0.13;
    c.beginPath();
    c.moveTo(cx - lHaut / 2, hautFut);
    c.lineTo(cx + lHaut / 2, hautFut);
    c.lineTo(cx + lBas / 2, basFut);
    c.lineTo(cx - lBas / 2, basFut);
    c.closePath();
    c.fill();
    const lumiere = c.createLinearGradient(cx - l / 2, 0, cx + l / 2, 0);
    lumiere.addColorStop(0, "rgba(255,255,255,0.32)");
    lumiere.addColorStop(0.45, "rgba(255,255,255,0.08)");
    lumiere.addColorStop(1, "rgba(255,255,255,0)");
    c.globalAlpha = 1;
    c.fillStyle = lumiere;
    c.fill();
    c.fillStyle = "#fff";
    c.strokeStyle = "#fff";
    const cannelures = 8;
    for (let i = 0; i <= cannelures; i += 1) {
      const t = i / cannelures;
      const bord = i === 0 || i === cannelures;
      c.globalAlpha = bord ? 0.85 : 0.5 + 0.25 * (1 - t);
      c.lineWidth = bord ? 2.4 : 1.6;
      c.beginPath();
      c.moveTo(cx - lHaut / 2 + lHaut * t, hautFut);
      c.lineTo(cx - lBas / 2 + lBas * t, basFut);
      c.stroke();
    }
    // Astragale sous le chapiteau.
    c.globalAlpha = 0.75;
    c.fillRect(cx - lHaut * 0.56, hautFut - 3, lHaut * 1.12, 5);

    // Base attique : tores et plinthe.
    c.globalAlpha = 0.72;
    c.beginPath();
    c.ellipse(cx, basFut + hBase * 0.18, lBas * 0.6, hBase * 0.16, 0, 0, Math.PI * 2);
    c.fill();
    c.globalAlpha = 0.5;
    c.beginPath();
    c.ellipse(cx, basFut + hBase * 0.45, lBas * 0.68, hBase * 0.18, 0, 0, Math.PI * 2);
    c.fill();
    c.globalAlpha = 0.62;
    c.fillRect(cx - lBas * 0.78, basFut + hBase * 0.62, lBas * 1.56, hBase * 0.38);

    // Chapiteau corinthien : corbeille, deux rangs de feuilles d'acanthe,
    // volutes aux angles, abaque.
    const hautCh = haut;
    const basCh = hautFut - 2;
    c.globalAlpha = 0.28;
    c.beginPath();
    c.moveTo(cx - lHaut * 0.55, basCh);
    c.lineTo(cx + lHaut * 0.55, basCh);
    c.lineTo(cx + l * 0.82, hautCh + hChapiteau * 0.14);
    c.lineTo(cx - l * 0.82, hautCh + hChapiteau * 0.14);
    c.closePath();
    c.fill();
    for (let rang = 0; rang < 2; rang += 1) {
      const y = basCh - hChapiteau * (0.2 + rang * 0.3);
      const etendue = lHaut * (0.62 + rang * 0.22);
      const feuilles = 5 + rang;
      for (let i = 0; i < feuilles; i += 1) {
        const x = cx - etendue + (2 * etendue * (i + 0.5)) / feuilles;
        c.globalAlpha = 0.55 + hasard() * 0.35;
        c.beginPath();
        c.ellipse(x, y, l * 0.085, hChapiteau * 0.17, (x - cx) / (l * 3), 0, Math.PI * 2);
        c.fill();
        // Pointe de la feuille, recourbée vers l'extérieur.
        c.beginPath();
        c.arc(x + (x - cx) * 0.12, y - hChapiteau * 0.17, l * 0.04, 0, Math.PI * 2);
        c.fill();
      }
    }
    c.lineWidth = 2;
    for (const sens of [-1, 1]) {
      const vx = cx + sens * l * 0.68;
      const vy = hautCh + hChapiteau * 0.26;
      c.globalAlpha = 0.85;
      for (let tour = 0; tour < 2; tour += 1) {
        c.beginPath();
        c.arc(vx, vy, l * (0.12 - tour * 0.05), 0, Math.PI * 2);
        c.stroke();
      }
    }
    c.globalAlpha = 0.8;
    c.fillRect(cx - l * 0.9, hautCh, l * 1.8, Math.max(4, hChapiteau * 0.1));

    // Ruine : chapiteau entamé, éclats au-dessus.
    c.save();
    c.globalCompositeOperation = "destination-out";
    for (let i = 0; i < ruine; i += 1) {
      c.globalAlpha = 0.5 + hasard() * 0.5;
      c.beginPath();
      c.arc(
        cx + (hasard() - 0.5) * l * 2,
        hautCh + hasard() * hChapiteau * 0.9,
        l * (0.06 + hasard() * 0.14),
        0,
        Math.PI * 2,
      );
      c.fill();
    }
    // Usure du fût : des manques le long des cannelures.
    for (let i = 0; i < 26; i += 1) {
      c.globalAlpha = 0.35 + hasard() * 0.4;
      c.fillRect(cx + (hasard() - 0.5) * l, hautFut + hasard() * (basFut - hautFut), 2 + hasard() * 4, 8 + hasard() * 40);
    }
    c.restore();
    c.globalAlpha = 0.6;
    for (let i = 0; i < ruine * 3; i += 1) {
      c.fillRect(cx + (hasard() - 0.5) * l * 2.4, hautCh - hasard() * hChapiteau * 0.35, 2, 2);
    }
    // Gravats au pied.
    for (let i = 0; i < 70; i += 1) {
      c.globalAlpha = 0.3 + hasard() * 0.5;
      c.fillRect(cx + (hasard() - 0.5) * l * 2.6, bas - hasard() * hBase * 0.4, 2, 2);
    }
    c.globalAlpha = 1;
  }

  function dessin() {
    const t = document.createElement("canvas");
    t.width = largeur;
    t.height = hauteur;
    const c = t.getContext("2d");
    c.fillStyle = "#fff";
    c.strokeStyle = "#fff";
    graine = 7;
    const L = largeur;
    const H = hauteur;
    // Colonnes tenues dans la zone toujours visible du portail, où l'image
    // est cadrée en cover jusqu'à un panneau de rapport 0,7 (environ 85 px
    // rognés de chaque côté sur 800) : chapiteaux entiers, à gauche comme à
    // droite (remarque de l'utilisateur du 2026-11-19).
    const grand = Math.min(L * 0.12, H * 0.12);
    const moyen = grand * 0.74;
    const sol = H * 0.985;
    colonne(c, L * 0.24, grand, -H * 0.03, sol, 18);
    colonne(c, L * 0.51, moyen, H * 0.17, sol, 12);
    colonne(c, L * 0.76, grand, -H * 0.03, sol, 16);
    return c.getImageData(0, 0, largeur, hauteur).data;
  }

  // ----- Trame -----

  function placer() {
    const donnees = dessin();
    graine = 11;
    const nouveaux = [];
    for (let y = PAS / 2; y < hauteur; y += PAS) {
      for (let x = PAS / 2; x < largeur; x += PAS) {
        const clarte = donnees[(Math.round(y) * largeur + Math.round(x)) * 4 + 3] / 255;
        if (clarte < 0.04 || hasard() > Math.pow(clarte, 0.8) * 1.05) {
          continue;
        }
        const teinte = clarte > 0.6 ? 2 : clarte > 0.3 ? 1 : 0;
        nouveaux.push({ x: x, y: y, hx: x, hy: y, vx: 0, vy: 0, teinte: teinte });
      }
    }
    points = nouveaux;
  }

  function dimensionner() {
    const boite = panneau.getBoundingClientRect();
    const l = Math.max(1, Math.round(boite.width));
    const h = Math.max(1, Math.round(boite.height));
    if (l === largeur && h === hauteur) {
      return;
    }
    largeur = l;
    hauteur = h;
    ratio = window.devicePixelRatio || 1;
    toile.width = Math.round(largeur * ratio);
    toile.height = Math.round(hauteur * ratio);
    placer();
    if (!reactif) {
      image();
    }
  }

  function image() {
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, largeur, hauteur);
    const actif = reactif && !reduit;
    const portee2 = PORTEE * PORTEE;
    const parTeinte = [[], [], []];
    for (const p of points) {
      let ax = (p.hx - p.x) * RESSORT;
      let ay = (p.hy - p.y) * RESSORT;
      if (actif) {
        const dx = p.x - souris.x;
        const dy = p.y - souris.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < portee2) {
          const d = Math.sqrt(d2) || 1;
          const force = (1 - d / PORTEE) * POUSSEE;
          ax += (dx / d) * force;
          ay += (dy / d) * force;
        }
      }
      p.vx = (p.vx + ax) * AMORTI;
      p.vy = (p.vy + ay) * AMORTI;
      p.x += p.vx;
      p.y += p.vy;
      parTeinte[p.teinte].push(p);
    }
    if (!actif) {
      for (const p of points) {
        p.x = p.hx;
        p.y = p.hy;
      }
    }
    for (let i = 0; i < 3; i += 1) {
      ctx.fillStyle = TEINTES[i];
      for (const p of parTeinte[i]) {
        ctx.fillRect(p.x - TAILLE / 2, p.y - TAILLE / 2, TAILLE, TAILLE);
      }
    }
    if (reactif) {
      window.requestAnimationFrame(image);
    }
  }

  // ----- Souris (seulement avec ?souris-active) -----

  if (reactif) {
    panneau.addEventListener("pointermove", function (e) {
      const boite = panneau.getBoundingClientRect();
      souris.x = e.clientX - boite.left;
      souris.y = e.clientY - boite.top;
    });
    panneau.addEventListener("pointerleave", function () {
      souris.x = -1e5;
      souris.y = -1e5;
    });
  }

  // Export en SVG pour le portail (?svg) : la même trame sur 800 x 900,
  // chaque point en trait de longueur nulle, coordonnées relatives.
  if (parametres.has("svg")) {
    largeur = 800;
    hauteur = 900;
    placer();
    const traces = ["", "", ""];
    const derniers = [null, null, null];
    for (const p of points) {
      const x = Math.round(p.hx);
      const y = Math.round(p.hy);
      const d = derniers[p.teinte];
      traces[p.teinte] += d ? "m" + (x - d[0]) + " " + (y - d[1]) + "h0" : "M" + x + " " + y + "h0";
      derniers[p.teinte] = [x, y];
    }
    const opacites = ["0.42", "0.68", "0.95"];
    let svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 900" preserveAspectRatio="xMidYMax slice">' +
      '<g fill="none" stroke="#E2E6DC" stroke-width="1.7" stroke-linecap="square">';
    for (let i = 0; i < 3; i += 1) {
      svg += '<path stroke-opacity="' + opacites[i] + '" d="' + traces[i] + '"/>';
    }
    svg += "</g></svg>";
    const sortie = document.createElement("pre");
    sortie.id = "svg-sortie";
    sortie.textContent = svg;
    document.body.appendChild(sortie);
    return;
  }

  dimensionner();
  if (window.ResizeObserver) {
    new ResizeObserver(dimensionner).observe(panneau);
  } else {
    window.addEventListener("resize", dimensionner);
  }
  if (reactif) {
    window.requestAnimationFrame(image);
  }
})();
