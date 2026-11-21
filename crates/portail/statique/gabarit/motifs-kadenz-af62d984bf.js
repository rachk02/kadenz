/* Motifs animés Kadenz pour les emplacements d'images de la maquette du portail.
   Repris de docs/marque/propositions-2026-11-18-ecrans-animes/generer.py (MOTIF A, A2, C,
   mode sombre), palette Anthracite. Directions retenues par l'utilisateur le 2026-11-18.

   <canvas data-motif="C|A2|A|S|P" [data-forme="fronton|colonne|arche|amphore|escalier|chapiteau|ruine"] data-cadre="plein|vignette" [data-piece="carre|quart|tri|disque"]
           [data-decalage="secondes"]>
   Le canevas suit la taille de son élément ; l'animation s'arrête hors de l'écran et,
   si l'utilisateur réduit les animations, une seule image fixe est dessinée. */
(function () {
  "use strict";
  var PI = Math.PI;
  var PIECES = {
    carre: new Path2D("M16 16 L120 16 L120 120 L16 120 Z"),
    quart: new Path2D("M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z"),
    tri: new Path2D("M16 136 L120 240 L16 240 Z"),
    disque: new Path2D("M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z")
  };
  var ORDRE = ["carre", "quart", "tri", "disque"];
  var CENTRE = { carre: [68, 68], quart: [188, 68], tri: [68, 188], disque: [188, 188] };
  var AMBRE = "#A7B58E"; // vert sauge de la page de connexion du portail (thème du 2026-11-21, ambre #e3a23b avant)
  // Palette Anthracite (mode sombre du logiciel)
  var K = { fond: "#17181A", points: "#4a4d52", motif: "#202224", motifClair: "#2A2C2E" }; // noir profond du portail (Anthracite avant : #1a1918, #22201e, #2c2a27)

  function clamp(x, a, b) { a = a === undefined ? 0 : a; b = b === undefined ? 1 : b; return Math.max(a, Math.min(b, x)); }
  function lin(t, a, d) { return clamp((t - a) / d); }
  function eOut(x) { return 1 - Math.pow(1 - x, 3); }
  function eInOut(x) { return x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2; }
  function eBack(x) { var c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2); }
  function hash(i, j) { return (((i * 73856093) ^ (j * 19349663)) >>> 0) % 4; }
  function dans(k, u, v) {
    switch (k) {
      case "carre": return u >= 16 && u <= 120 && v >= 16 && v <= 120;
      case "quart": return u >= 136 && v <= 120 && Math.pow(u - 136, 2) + Math.pow(v - 120, 2) <= 105.56 * 105.56;
      case "tri": return u >= 16 && v <= 240 && v >= 136 && (u - 16) <= (v - 136);
      case "disque": return Math.pow(u - 188, 2) + Math.pow(v - 188, 2) <= 53.56 * 53.56;
    }
    return false;
  }
  function unePiece(c, k, cx, cy, s, couleur, e) {
    e = e === undefined ? 1 : e; if (e <= 0.001) return;
    var p = CENTRE[k];
    c.save(); c.translate(cx, cy); c.scale(s / 104 * e, s / 104 * e); c.translate(-p[0], -p[1]);
    c.fillStyle = couleur; c.fill(PIECES[k]); c.restore();
  }
  // La pièce vedette au centre du cadre (placement « vignette » des écrans animés)
  function placement(w, h, piece) {
    var cell = 0.72 * Math.min(w, h), cx = w / 2, cy = h / 2, S = cell * 256 / 104, p = CENTRE[piece];
    return { cell: cell, S: S, ox: cx - p[0] / 256 * S, oy: cy - p[1] / 256 * S, piece: piece };
  }

  // ----- P Pierres : architecture antique en points, facture des colonnes de la page de
  // connexion (docs/marque/propositions-2026-11-19-portail/piliers.js) : la forme est dessinée
  // en clarté sur une toile cachée, puis tramée, un point par case gardé selon la clarté.
  // Un quart de disque ambre, la pièce du symbole, se pose en points derrière la pierre.
  // <canvas data-motif="P" data-forme="fronton|colonne|arche|amphore|escalier|chapiteau|ruine">
  function alea(graine) {
    var g = graine >>> 0;
    return function () { g = (g * 1664525 + 1013904223) % 4294967296; return g / 4294967296; };
  }
  // Fût cannelé avec sa base attique, du haut `haut` au sol `bas`, largeur `l` en bas
  function fut(c, cx, l, haut, bas, r, k) {
    var hBase = Math.min((bas - haut) * 0.07, l * 0.6), basFut = bas - hBase, lHaut = l * 0.86;
    c.globalAlpha = 0.13;
    c.beginPath(); c.moveTo(cx - lHaut / 2, haut); c.lineTo(cx + lHaut / 2, haut);
    c.lineTo(cx + l / 2, basFut); c.lineTo(cx - l / 2, basFut); c.closePath(); c.fill();
    var lumiere = c.createLinearGradient(cx - l / 2, 0, cx + l / 2, 0);
    lumiere.addColorStop(0, "rgba(255,255,255,0.32)"); lumiere.addColorStop(0.45, "rgba(255,255,255,0.08)");
    lumiere.addColorStop(1, "rgba(255,255,255,0)");
    c.globalAlpha = 1; c.fillStyle = lumiere; c.fill(); c.fillStyle = "#fff";
    for (var i = 0; i <= 8; i++) {
      var t = i / 8, bord = i === 0 || i === 8;
      c.globalAlpha = bord ? 0.85 : 0.5 + 0.25 * (1 - t); c.lineWidth = (bord ? 2.4 : 1.6) * k;
      c.beginPath(); c.moveTo(cx - lHaut / 2 + lHaut * t, haut); c.lineTo(cx - l / 2 + l * t, basFut); c.stroke();
    }
    c.globalAlpha = 0.72; c.beginPath(); c.ellipse(cx, basFut + hBase * 0.18, l * 0.6, hBase * 0.16, 0, 0, 2 * PI); c.fill();
    c.globalAlpha = 0.5; c.beginPath(); c.ellipse(cx, basFut + hBase * 0.45, l * 0.68, hBase * 0.18, 0, 0, 2 * PI); c.fill();
    c.globalAlpha = 0.62; c.fillRect(cx - l * 0.78, basFut + hBase * 0.62, l * 1.56, hBase * 0.38);
    // Usure le long des cannelures
    c.save(); c.globalCompositeOperation = "destination-out";
    for (var u = 0; u < 22; u++) {
      c.globalAlpha = 0.35 + r() * 0.4;
      c.fillRect(cx + (r() - 0.5) * l, haut + r() * (basFut - haut), (2 + r() * 4) * k, (8 + r() * 40) * k);
    }
    c.restore(); c.globalAlpha = 1;
  }
  // Chapiteau corinthien posé sur le fût : corbeille, deux rangs d'acanthe, volutes, abaque
  function corinthien(c, cx, l, haut, hCh, r, k) {
    var lHaut = l * 0.86, basCh = haut + hCh;
    c.globalAlpha = 0.75; c.fillRect(cx - lHaut * 0.56, basCh - 1 * k, lHaut * 1.12, 5 * k);
    c.globalAlpha = 0.28; c.beginPath(); c.moveTo(cx - lHaut * 0.55, basCh); c.lineTo(cx + lHaut * 0.55, basCh);
    c.lineTo(cx + l * 0.82, haut + hCh * 0.14); c.lineTo(cx - l * 0.82, haut + hCh * 0.14); c.closePath(); c.fill();
    for (var rang = 0; rang < 2; rang++) {
      var y = basCh - hCh * (0.2 + rang * 0.3), et = lHaut * (0.62 + rang * 0.22), n = 5 + rang;
      for (var i = 0; i < n; i++) {
        var x = cx - et + 2 * et * (i + 0.5) / n;
        c.globalAlpha = 0.55 + r() * 0.35;
        c.beginPath(); c.ellipse(x, y, l * 0.085, hCh * 0.17, (x - cx) / (l * 3), 0, 2 * PI); c.fill();
        c.beginPath(); c.arc(x + (x - cx) * 0.12, y - hCh * 0.17, l * 0.04, 0, 2 * PI); c.fill();
      }
    }
    c.lineWidth = 2 * k;
    [-1, 1].forEach(function (s) {
      c.globalAlpha = 0.85;
      for (var t = 0; t < 2; t++) { c.beginPath(); c.arc(cx + s * l * 0.68, haut + hCh * 0.26, l * (0.12 - t * 0.05), 0, 2 * PI); c.stroke(); }
    });
    c.globalAlpha = 0.8; c.fillRect(cx - l * 0.9, haut, l * 1.8, Math.max(4 * k, hCh * 0.1));
    c.globalAlpha = 1;
  }
  // Ruine : manques ronds dans la pierre et éclats autour
  function entamer(c, cx, cy, l, hz, n, r) {
    c.save(); c.globalCompositeOperation = "destination-out";
    for (var i = 0; i < n; i++) {
      c.globalAlpha = 0.5 + r() * 0.5; c.beginPath();
      c.arc(cx + (r() - 0.5) * l * 2, cy + r() * hz, l * (0.06 + r() * 0.14), 0, 2 * PI); c.fill();
    }
    c.restore();
    c.globalAlpha = 0.6;
    for (var j = 0; j < n * 3; j++) c.fillRect(cx + (r() - 0.5) * l * 2.4, cy - r() * hz * 0.35, 2, 2);
    c.globalAlpha = 1;
  }
  function gravats(c, x0, x1, sol, n, r) {
    for (var i = 0; i < n; i++) { c.globalAlpha = 0.3 + r() * 0.5; c.fillRect(x0 + r() * (x1 - x0), sol - r() * 6, 2, 2); }
    c.globalAlpha = 1;
  }
  // Pierre de taille : fond pâle, arêtes claires, éclairée de gauche
  function bloc(c, x, y, w, h, k, fond) {
    c.globalAlpha = fond === undefined ? 0.16 : fond; c.fillRect(x, y, w, h);
    c.globalAlpha = 0.8; c.fillRect(x, y, w, 2 * k);
    c.globalAlpha = 0.45; c.fillRect(x, y, 2 * k, h);
    c.globalAlpha = 1;
  }
  function colonneDorique(c, cx, l, haut, bas, r, k) {
    var hCh = l * 0.5;
    fut(c, cx, l, haut + hCh, bas, r, k);
    c.globalAlpha = 0.55; c.beginPath(); c.ellipse(cx, haut + hCh * 0.72, l * 0.58, hCh * 0.3, 0, 0, 2 * PI); c.fill();
    bloc(c, cx - l * 0.66, haut, l * 1.32, hCh * 0.42, k, 0.5);
  }
  var FORMES = {
    colonne: function (c, w, h, r, k) {
      var l = Math.min(w * 0.26, h * 0.17), haut = h * 0.08, sol = h * 0.95, hCh = Math.min(h * 0.17, l * 1.55);
      fut(c, w / 2, l, haut + hCh, sol, r, k); corinthien(c, w / 2, l, haut, hCh, r, k);
      entamer(c, w / 2, haut, l, hCh * 0.9, 10, r); gravats(c, w / 2 - l * 1.3, w / 2 + l * 1.3, sol, 60, r);
      return { x: w * 0.12, y: h * 0.62, R: Math.min(w, h) * 0.34 };
    },
    chapiteau: function (c, w, h, r, k) {
      var l = Math.min(w * 0.42, h * 0.5), haut = h * 0.2, hCh = l * 1.2;
      fut(c, w / 2, l, haut + hCh, h * 1.6, r, k); corinthien(c, w / 2, l, haut, hCh, r, k);
      entamer(c, w / 2, haut, l, hCh * 0.8, 14, r);
      return { x: w * 0.06, y: h * 0.56, R: Math.min(w, h) * 0.38 };
    },
    fronton: function (c, w, h, r, k) {
      var L = Math.min(w * 0.84, h * 1.05), x0 = (w - L) / 2, sol = h * 0.9, hMarche = L * 0.035;
      for (var m = 0; m < 3; m++) bloc(c, x0 - L * 0.04 + m * L * 0.03, sol - (m + 1) * hMarche, L * 1.08 - m * L * 0.06, hMarche, k, 0.2);
      var bas = sol - 3 * hMarche, hCol = L * 0.42, haut = bas - hCol, lc = L * 0.085;
      for (var i = 0; i < 4; i++) colonneDorique(c, x0 + L * (0.11 + i * 0.26), lc, haut, bas, r, k);
      var hA = L * 0.05; bloc(c, x0, haut - hA, L, hA, k, 0.3);
      var hF = L * 0.06, yF = haut - hA - hF; c.globalAlpha = 0.14; c.fillRect(x0, yF, L, hF);
      for (var t = 0; t < 9; t++) { c.globalAlpha = 0.7; c.fillRect(x0 + L * (0.03 + t * 0.117), yF + 2 * k, L * 0.03, hF - 4 * k); }
      bloc(c, x0 - L * 0.03, yF - L * 0.02, L * 1.06, L * 0.02, k, 0.6);
      var yB = yF - L * 0.02, hP = L * 0.2;
      c.globalAlpha = 0.1; c.beginPath(); c.moveTo(x0 - L * 0.03, yB); c.lineTo(w / 2, yB - hP); c.lineTo(x0 + L * 1.03, yB); c.closePath(); c.fill();
      c.globalAlpha = 0.85; c.lineWidth = 3.2 * k; c.stroke();
      entamer(c, x0 + L * 0.8, yB - hP * 0.5, L * 0.12, hP * 0.6, 9, r);
      gravats(c, x0, x0 + L, sol, 70, r);
      return { x: w * 0.55, y: yB - hP * 0.35, R: Math.min(w, h) * 0.42 };
    },
    arche: function (c, w, h, r, k) {
      var sol = h * 0.94, Lp = Math.min(w * 0.17, h * 0.13), ouv = Math.min(w * 0.46, h * 0.42);
      var xg = w / 2 - ouv / 2 - Lp, xd = w / 2 + ouv / 2, hP = h * 0.38, yI = sol - hP;
      [xg, xd].forEach(function (x) {
        bloc(c, x, yI, Lp, hP, k, 0.18);
        for (var j = 1; j < 6; j++) { c.globalAlpha = 0.5; c.fillRect(x, yI + j * hP / 6, Lp, 1.6 * k); }
        bloc(c, x - Lp * 0.12, yI - Lp * 0.18, Lp * 1.24, Lp * 0.18, k, 0.55);
      });
      var R1 = ouv / 2, R2 = R1 + Lp * 0.9, cy = yI - Lp * 0.18, n = 13;
      for (var i = 0; i < n; i++) {
        var a0 = PI + i * PI / n + 0.012, a1 = PI + (i + 1) * PI / n - 0.012, cle = i === (n - 1) / 2;
        c.globalAlpha = cle ? 0.42 : 0.17 + 0.1 * Math.cos(a0 + PI * 0.75);
        c.beginPath(); c.arc(w / 2, cy, cle ? R2 * 1.06 : R2, a0, a1); c.arc(w / 2, cy, R1, a1, a0, true); c.closePath(); c.fill();
        c.globalAlpha = 0.8; c.lineWidth = 1.8 * k; c.stroke();
      }
      entamer(c, w / 2 + R2 * 0.7, cy - R2 * 0.6, Lp * 0.6, Lp, 8, r);
      gravats(c, xg - Lp * 0.5, xd + Lp * 1.5, sol, 60, r);
      return { x: w / 2 - R1 * 0.7, y: sol, R: R1 * 1.25 };
    },
    amphore: function (c, w, h, r, k) {
      var H = h * 0.8, cx = w / 2, y0 = h * 0.1, L = Math.min(w * 0.5, H * 0.56);
      c.beginPath();
      c.moveTo(cx - L * 0.16, y0); c.lineTo(cx + L * 0.16, y0);
      c.bezierCurveTo(cx + L * 0.12, y0 + H * 0.14, cx + L * 0.14, y0 + H * 0.2, cx + L * 0.4, y0 + H * 0.3);
      c.bezierCurveTo(cx + L * 0.62, y0 + H * 0.42, cx + L * 0.44, y0 + H * 0.78, cx + L * 0.1, y0 + H * 0.94);
      c.lineTo(cx + L * 0.14, y0 + H); c.lineTo(cx - L * 0.14, y0 + H); c.lineTo(cx - L * 0.1, y0 + H * 0.94);
      c.bezierCurveTo(cx - L * 0.44, y0 + H * 0.78, cx - L * 0.62, y0 + H * 0.42, cx - L * 0.4, y0 + H * 0.3);
      c.bezierCurveTo(cx - L * 0.14, y0 + H * 0.2, cx - L * 0.12, y0 + H * 0.14, cx - L * 0.16, y0);
      c.closePath();
      c.globalAlpha = 0.14; c.fill();
      var lum = c.createLinearGradient(cx - L * 0.5, 0, cx + L * 0.5, 0);
      lum.addColorStop(0, "rgba(255,255,255,0.4)"); lum.addColorStop(0.4, "rgba(255,255,255,0.12)"); lum.addColorStop(1, "rgba(255,255,255,0)");
      c.globalAlpha = 1; c.fillStyle = lum; c.fill(); c.fillStyle = "#fff";
      c.globalAlpha = 0.85; c.lineWidth = 2.4 * k; c.stroke();
      c.save(); c.clip();
      [0.36, 0.4, 0.62, 0.66].forEach(function (f) { c.globalAlpha = 0.7; c.fillRect(cx - L, y0 + H * f, 2 * L, 2.2 * k); });
      for (var i = 0; i < 9; i++) { c.globalAlpha = 0.55; c.beginPath(); c.arc(cx - L * 0.4 + i * L * 0.1, y0 + H * 0.51, L * 0.022, 0, 2 * PI); c.fill(); }
      c.restore();
      c.lineWidth = 2.6 * k; c.globalAlpha = 0.8;
      [-1, 1].forEach(function (s) {
        c.beginPath(); c.moveTo(cx + s * L * 0.15, y0 + H * 0.06);
        c.bezierCurveTo(cx + s * L * 0.42, y0 + H * 0.02, cx + s * L * 0.46, y0 + H * 0.2, cx + s * L * 0.34, y0 + H * 0.27); c.stroke();
      });
      bloc(c, cx - L * 0.2, y0 - H * 0.02, L * 0.4, H * 0.03, k, 0.6);
      entamer(c, cx + L * 0.25, y0 + H * 0.2, L * 0.2, H * 0.2, 6, r);
      gravats(c, cx - L * 0.7, cx + L * 0.7, y0 + H + h * 0.04, 50, r);
      return { x: w * 0.6, y: h * 0.52, R: Math.min(w, h) * 0.4 };
    },
    escalier: function (c, w, h, r, k) {
      var n = 7, sol = h * 0.95, hM = h * 0.075, x0 = w * 0.06, pas = w * 0.1;
      for (var i = 0; i < n; i++) {
        var x = x0 + i * pas, y = sol - (i + 1) * hM;
        c.globalAlpha = 0.12 + i * 0.012; c.fillRect(x, y, w - x, hM);
        c.globalAlpha = 0.85; c.fillRect(x, y, w - x, 2.4 * k);
        c.globalAlpha = 0.4; c.fillRect(x, y, 1.6 * k, hM);
        for (var j = 0; j < 3; j++) { c.globalAlpha = 0.3; c.fillRect(x + (r() * 0.8 + 0.1) * (w - x), y + 2 * k, 1.4 * k, hM - 2 * k); }
      }
      var top = sol - n * hM, l = Math.min(w * 0.09, h * 0.08);
      colonneDorique(c, x0 + (n - 1) * pas + pas * 1.2, l, top - h * 0.36, top, r, k);
      entamer(c, x0 + (n - 1) * pas + pas * 1.2, top - h * 0.36, l, h * 0.1, 7, r);
      gravats(c, x0, w, sol, 40, r);
      return { x: w * 0.08, y: h * 0.5, R: Math.min(w, h) * 0.36 };
    },
    ruine: function (c, w, h, r, k) {
      var sol = h * 0.93, l = Math.min(w * 0.14, h * 0.12);
      [[0.2, 0.18, 0.7], [0.5, 0.42, 0.85], [0.8, 0.3, 0.6]].forEach(function (d) {
        var cx = w * d[0], haut = h * d[1];
        fut(c, cx, l * d[2] / 0.7, haut, sol, r, k);
        c.save(); c.globalCompositeOperation = "destination-out"; c.globalAlpha = 1; c.beginPath();
        c.moveTo(cx - l, haut - 2); c.lineTo(cx + l, haut - 2); c.lineTo(cx + l, haut + l * 0.3);
        for (var s = 0; s <= 6; s++) c.lineTo(cx + l - s * l / 3, haut + (s % 2 ? l * 0.05 : l * 0.3 * r()));
        c.closePath(); c.fill(); c.restore();
        entamer(c, cx, haut, l * 0.6, l * 0.4, 5, r);
      });
      // Tambour tombé au sol
      var ty = sol - l * 0.5;
      c.globalAlpha = 0.18; c.fillRect(w * 0.3, ty - l * 0.45, w * 0.22, l * 0.9);
      c.globalAlpha = 0.8; c.lineWidth = 2 * k; c.strokeRect(w * 0.3, ty - l * 0.45, w * 0.22, l * 0.9);
      c.globalAlpha = 0.5; c.beginPath(); c.ellipse(w * 0.52, ty, l * 0.18, l * 0.45, 0, 0, 2 * PI); c.fill();
      gravats(c, w * 0.05, w * 0.95, sol, 110, r);
      return { x: w * 0.6, y: h * 0.6, R: Math.min(w, h) * 0.45 };
    }
  };
  // Trame des colonnes de la connexion : un point carré de 1,7 px par case de 4 px, gardé
  // selon la clarté du dessin (puissance 0,8), en trois teintes selon la clarté
  var PAS_P = 4, TAILLE_P = 1.7;
  function tramer(da, w, h, r, garder) {
    var pts = [];
    for (var y = PAS_P / 2; y < h; y += PAS_P) for (var x = PAS_P / 2; x < w; x += PAS_P) {
      var cl = da[(Math.round(y - 0.5) * w + Math.round(x - 0.5)) * 4 + 3] / 255;
      if (cl < 0.04 || r() > Math.pow(cl, 0.8) * 1.05 || (garder && !garder(x, y))) continue;
      pts.push([x, y, cl > 0.6 ? 2 : cl > 0.3 ? 1 : 0, r()]);
    }
    return pts;
  }
  function toileCachee(w, h) {
    var a = document.createElement("canvas"); a.width = w; a.height = h;
    var c = a.getContext("2d"); c.fillStyle = "#fff"; c.strokeStyle = "#fff"; return c;
  }
  // Modelé d'une pièce du symbole, comme le fût des colonnes : fond pâle, lumière venue de
  // gauche, arêtes claires, stries de taille, usure et éclats, sur le chemin `chemin` (Path2D).
  function modeler(c, chemin, x0, y0, L, r, k) {
    c.save();
    c.globalAlpha = 0.13; c.fill(chemin);
    var lum = c.createLinearGradient(x0, y0, x0 + L, y0 + L);
    lum.addColorStop(0, "rgba(255,255,255,0.36)"); lum.addColorStop(0.5, "rgba(255,255,255,0.1)"); lum.addColorStop(1, "rgba(255,255,255,0)");
    c.globalAlpha = 1; c.fillStyle = lum; c.fill(chemin); c.fillStyle = "#fff";
    c.globalAlpha = 0.85; c.lineWidth = 2.4 * k; c.stroke(chemin);
    c.clip(chemin);
    for (var i = 0; i < 9; i++) {
      var t = (i + 0.5) / 9; c.globalAlpha = 0.3 + 0.25 * (1 - t); c.lineWidth = 1.6 * k;
      c.beginPath(); c.moveTo(x0 + L * t, y0 - 2); c.lineTo(x0 + L * t - L * 0.06, y0 + L + 2); c.stroke();
    }
    c.globalCompositeOperation = "destination-out";
    for (var u = 0; u < 18; u++) {
      c.globalAlpha = 0.35 + r() * 0.4;
      c.fillRect(x0 + r() * L, y0 + r() * L, (2 + r() * 4) * k, (6 + r() * 26) * k);
    }
    for (var e = 0; e < 5; e++) {
      c.globalAlpha = 0.5 + r() * 0.5; c.beginPath();
      c.arc(x0 + r() * L, y0 + (r() < 0.5 ? r() * 0.15 : 0.85 + r() * 0.15) * L, L * (0.03 + r() * 0.06), 0, 2 * PI); c.fill();
    }
    c.restore(); c.globalAlpha = 1;
  }
  function trameP(w, h, forme, graine) {
    var k = Math.max(1, Math.min(w, h) / 400), r = alea(graine); // traits jamais plus fins que dans les colonnes
    var c = toileCachee(w, h);
    var q = (FORMES[forme] || FORMES.colonne)(c, w, h, r, k);
    var pierre = c.getImageData(0, 0, w, h).data;
    // Quart de disque ambre derrière la pierre, de même modelé : angle droit en bas à gauche
    var b = toileCachee(w, h);
    var quart = new Path2D(); quart.moveTo(q.x, q.y); quart.lineTo(q.x, q.y - q.R); quart.arc(q.x, q.y, q.R, -PI / 2, 0); quart.closePath();
    modeler(b, quart, q.x, q.y - q.R, q.R, r, k);
    var ambre = tramer(b.getImageData(0, 0, w, h).data, w, h, r, function (x, y) {
      return pierre[(Math.round(y - 0.5) * w + Math.round(x - 0.5)) * 4 + 3] < 10;
    });
    return { points: tramer(pierre, w, h, r), ambre: ambre };
  }
  var TEINTES_P = ["rgba(214,218,208,0.42)", "rgba(214,218,208,0.68)", "rgba(232,235,228,0.95)"];
  var TEINTES_AMBRE = ["rgba(167,181,142,0.42)", "rgba(167,181,142,0.68)", "rgba(188,201,164,0.95)"]; // sauge (nom gardé)
  // Pose des points par teinte ; `vu(pt)` dit si le point est déjà monté
  function poser(c, pts, teintes, vu) {
    var lots = [[], [], []];
    for (var i = 0; i < pts.length; i++) if (!vu || vu(pts[i])) lots[pts[i][2]].push(pts[i]);
    for (var t = 0; t < 3; t++) {
      c.fillStyle = teintes[t];
      for (var j = 0; j < lots[t].length; j++) c.fillRect(lots[t][j][0] - TAILLE_P / 2, lots[t][j][1] - TAILLE_P / 2, TAILLE_P, TAILLE_P);
    }
  }

  var MOTIF = {
    // P Pierres : la pierre se monte en points du sol vers le haut, puis le quart ambre ;
    // une fois monté, le dessin reste fixe, comme les colonnes de la connexion
    P: function (c, w, h, T, p) {
      var d = p.trame, g = lin(T, 0.15, 2.2), ga = lin(T, 1.4, 1.4);
      if (ga > 0) poser(c, d.ambre, TEINTES_AMBRE, ga >= 1 ? null : function (pt) { return ga * 1.2 > pt[3]; });
      poser(c, d.points, TEINTES_P, g >= 1 ? null : function (pt) { return g * 1.25 >= (1 - pt[1] / h) * 0.85 + pt[3] * 0.25; });
    },
    // A Trame : la pièce en points ambre sur un champ de points, avec l'onde de lumière
    A: function (c, w, h, T, p) {
      var pas = Math.max(10, w / (p.cadre === "vignette" ? 9 : 36)), g = eOut(lin(T, 0.2, 1.4));
      for (var y = pas / 2; y < h; y += pas) for (var x = pas / 2; x < w; x += pas) {
        var u = (x - p.ox) / p.S * 256, v = (y - p.oy) / p.S * 256;
        var onde = 0.5 + 0.5 * Math.sin((x * 0.9 + y * 0.6) * 0.011 - T * 2.4);
        if (dans(p.piece, u, v) && g > 0.01) {
          var d = (u + v) / 512, lum = Math.min(1, Math.max(0.42, 0.3 + d * 1.2));
          c.fillStyle = AMBRE; c.beginPath(); c.arc(x, y, pas * 0.5 * lum * g * (0.86 + 0.14 * onde), 0, 2 * PI); c.fill(); continue;
        }
        c.fillStyle = K.points; c.beginPath(); c.arc(x, y, pas * (0.09 + 0.07 * onde), 0, 2 * PI); c.fill();
      }
    },
    // A2 Trame de pièces : motif Quatre pièces, la pièce vedette se compose en ambre
    A2: function (c, w, h, T, p) {
      var cell = p.cadre === "vignette" ? Math.max(w, h) / 6 : w / 14, m = cell / 2;
      for (var j = 0; j * cell < h; j++) for (var i = 0; i * cell < w; i++) {
        var cx = i * cell + m, cy = j * cell + m, kk = ORDRE[(i + j) % 4];
        var u = (cx - p.ox) / p.S * 256, v = (cy - p.oy) / p.S * 256;
        if (dans(p.piece, u, v)) {
          var pc = CENTRE[p.piece], r = clamp(((u - pc[0]) + (v - pc[1])) / 208 + 0.5);
          var e = eBack(clamp((lin(T, 0.2, 1.4) * 1.25 - r) / 0.25));
          if (e > 0.01) { unePiece(c, kk, cx, cy, cell * 0.86, AMBRE, e); continue; }
        }
        var l = 0.5 + 0.5 * Math.sin((i + j) * 0.45 - T * PI / 2);
        unePiece(c, kk, cx, cy, cell * 0.86, l > 0.8 ? K.motifClair : K.motif);
      }
    },
    // S Symbole du pied, rendu comme les colonnes de la connexion : chaque pièce modelée
    // (lumière, arêtes, stries, usure) puis tramée ; le quart en ambre, les trois autres en
    // teintes de pierre. Les pièces se montent l'une après l'autre quand le pied apparaît.
    S: function (c, w, h, T, p) {
      if (!p.trameS) {
        // En mobile, le cadre du pied déborde l'écran (repris de l'origine) : le symbole se
        // centre sur la partie visible du cadre
        var v0 = Math.max(0, -p.gauche), v1 = Math.min(w, p.vue - p.gauche);
        if (!(v1 > v0)) { v0 = 0; v1 = w; }
        var S = Math.min(h * 0.9, (v1 - v0) * 0.9), ox = (v0 + v1 - S) / 2, oy = (h - S) / 2, k = Math.max(0.5, S / 400), r = alea(31);
        var cs = toileCachee(w, h), ca = toileCachee(w, h);
        ORDRE.forEach(function (kk) {
          var cible = kk === "quart" ? ca : cs, b = { carre: [16, 16, 104], quart: [136, 14, 106], tri: [16, 136, 104], disque: [134.44, 134.44, 107.12] }[kk];
          var chemin = new Path2D(); chemin.addPath(PIECES[kk], new DOMMatrix([S / 256, 0, 0, S / 256, ox, oy]));
          modeler(cible, chemin, ox + b[0] / 256 * S, oy + b[1] / 256 * S, b[2] / 256 * S, r, k);
        });
        var piece = function (x, y) { var u = (x - ox) / S * 256, v = (y - oy) / S * 256; for (var q = 0; q < 4; q++) if (dans(ORDRE[q], u, v)) return q; return -1; };
        var ps = tramer(cs.getImageData(0, 0, w, h).data, w, h, r), pa = tramer(ca.getImageData(0, 0, w, h).data, w, h, r);
        ps.concat(pa).forEach(function (pt) { pt.push(piece(pt[0], pt[1])); });
        p.trameS = { pierre: ps, ambre: pa };
      }
      var cr = ORDRE.map(function (k, i) { return lin(T, 0.3 + i * 0.4, 0.9); });
      var vu = function (pt) { var q = pt[4], e = q < 0 ? 1 : cr[q]; return e >= 1 || e * 1.2 > pt[3]; };
      poser(c, p.trameS.pierre, TEINTES_P, vu);
      poser(c, p.trameS.ambre, TEINTES_AMBRE, vu);
    },
    // C Tournant : les quarts de disque tournent d'un quart de tour au passage d'une onde ;
    // un seul quart en ambre
    C: function (c, w, h, T, p) {
      var cell = p.cadre === "vignette" ? w / 3 : w / 7;
      var m = cell / 2, R = cell * 0.96, cols = Math.ceil(w / cell), rows = Math.ceil(h / cell);
      var ai = p.cadre === "vignette" ? 1 : cols - 3, aj = Math.floor(rows * 0.6);
      // data-ambre-etroit="colonne,fraction de hauteur" : place du quart ambre sous 768 px
      if (p.ambreEtroit && w < 768) { ai = p.ambreEtroit[0]; aj = Math.floor(rows * p.ambreEtroit[1]); }
      for (var j = 0; j < rows; j++) for (var i = 0; i < cols; i++) {
        var cx = i * cell + m, cy = j * cell + m;
        var phase = T / 1.5 - (i + j) * 0.06, n = Math.floor(phase), fr = phase - n;
        var rot = (hash(i, j) + n + eInOut(clamp(fr / 0.35))) * PI / 2;
        c.save(); c.translate(cx, cy); c.rotate(rot); c.fillStyle = (i === ai && j === aj) ? AMBRE : K.motif;
        c.beginPath(); c.moveTo(-m, m); c.lineTo(-m, m - R); c.arc(-m, m, R, -PI / 2, 0); c.closePath(); c.fill(); c.restore();
      }
    }
  };

  var reduit = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  var toiles = [];
  var debut = performance.now();

  function dimensionner(t) {
    var r = t.el.getBoundingClientRect(), dpr = Math.min(2, window.devicePixelRatio || 1);
    var w = Math.max(1, Math.round(r.width)), h = Math.max(1, Math.round(r.height));
    if (w === t.w && h === t.h) return;
    t.w = w; t.h = h; t.el.width = Math.round(w * dpr); t.el.height = Math.round(h * dpr); t.dpr = dpr;
    t.p = placement(w, h, t.piece); t.p.cadre = t.cadre;
    t.p.gauche = r.left; t.p.vue = document.documentElement.clientWidth;
    if (t.el.dataset.ambreEtroit) t.p.ambreEtroit = t.el.dataset.ambreEtroit.split(",").map(parseFloat);
    if (t.motif === "P") t.p.trame = trameP(w, h, t.forme, 7 + Math.round(t.decalage * 100));
  }
  function dessiner(t, T) {
    dimensionner(t);
    var c = t.ctx;
    c.setTransform(t.dpr, 0, 0, t.dpr, 0, 0);
    // Le symbole du pied se pose sans fond, sur le fond de la page ; les autres motifs
    // remplissent leur cadre
    if (t.motif === "S") c.clearRect(0, 0, t.w, t.h);
    else { c.fillStyle = K.fond; c.fillRect(0, 0, t.w, t.h); }
    // data-depart="vue" : le temps du motif part de sa première apparition à l'écran
    if (t.depart === "vue" && !reduit) {
      if (t.t0 === undefined) { if (!t.visible) T = 0; else t.t0 = T; }
      T = t.t0 === undefined ? 0 : T - t.t0;
    }
    MOTIF[t.motif](c, t.w, t.h, T + t.decalage, t.p);
    if (!t.pret) { t.pret = true; t.el.classList.add("is-loaded"); }
  }

  var io = "IntersectionObserver" in window ? new IntersectionObserver(function (entrees) {
    entrees.forEach(function (e) {
      toiles.forEach(function (t) { if (t.el === e.target) t.visible = e.isIntersecting; });
    });
  }) : null;

  function ajouter(el) {
    var motif = el.dataset.motif;
    if (!MOTIF[motif] || toiles.some(function (t) { return t.el === el; })) return;
    var t = {
      el: el, ctx: el.getContext("2d"), motif: motif, cadre: el.dataset.cadre || "plein",
      piece: el.dataset.piece || "quart", decalage: parseFloat(el.dataset.decalage || "0") || 0,
      depart: el.dataset.depart || "", forme: el.dataset.forme || "colonne",
      visible: !io, w: 0, h: 0, dpr: 1, p: null, pret: false
    };
    toiles.push(t);
    if (io) io.observe(el);
    if (reduit) dessiner(t, 6);
  }
  document.querySelectorAll("canvas[data-motif]").forEach(ajouter);

  // Les carrousels infinis de la page clonent leurs cartes après ce script : une copie de
  // canevas arrive vide, elle est prise en charge ici comme les autres.
  if ("MutationObserver" in window) {
    new MutationObserver(function (mutations) {
      mutations.forEach(function (m) {
        m.addedNodes.forEach(function (n) {
          if (n.nodeType !== 1) return;
          if (n.matches("canvas[data-motif]")) ajouter(n);
          n.querySelectorAll("canvas[data-motif]").forEach(ajouter);
        });
      });
    }).observe(document.body, { childList: true, subtree: true });
  }

  // Image fixe (animations réduites) : l'état composé, à 6 s
  if (reduit) {
    addEventListener("resize", function () { toiles.forEach(function (t) { dessiner(t, 6); }); });
    return;
  }
  function boucle(maintenant) {
    var T = (maintenant - debut) / 1000;
    // Le bouton pause d'une galerie pose .is-paused (ou data-da-paused) : ses motifs s'arrêtent
    toiles.forEach(function (t) {
      if (!t.pret || (t.visible && !t.el.closest(".is-paused, [data-da-paused]"))) dessiner(t, T);
    });
    requestAnimationFrame(boucle);
  }
  requestAnimationFrame(boucle);
})();
