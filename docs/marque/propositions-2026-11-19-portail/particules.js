// Portail des licences : illustration en particules de la connexion A,
// proposée le 2026-11-19 (kadenz-e6). Les points dessinent une figure ; la
// souris les écarte, puis ils reviennent doucement à leur place (ressort
// amorti). Changer de figure fait glisser les points vers la nouvelle.
//
// Figures dessinées dans une boîte de 100 sur 100, en trois calques :
// « plein » (points serrés, Lavande), « accent » (Orange), « trame » (points
// espacés, Violet clair estompé). Un point de calque plein masque les deux
// autres, un point d'accent masque la trame.
//
// Paramètres de l'adresse : #parthenon (figure), ?capture (sans barre de
// choix, points posés d'emblée), ?souris=0.6,0.45 (souris figée, en
// fraction du panneau, pour une capture de la perturbation).

(function () {
  "use strict";

  const toile = document.getElementById("particules");
  if (!toile || !toile.getContext) {
    return;
  }
  const ctx = toile.getContext("2d");
  const panneau = toile.parentElement;
  const legende = document.getElementById("legende-figure");
  const parametres = new URLSearchParams(window.location.search);
  const capture = parametres.has("capture");
  const reduit = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (capture) {
    document.body.classList.add("capture");
  }

  const COULEURS = {
    plein: "#D9C8F5",
    accent: "#F56A00",
    trame: "rgba(183, 148, 240, 0.26)",
  };
  const PAS = { plein: 4.5, accent: 4.5, trame: 9 };
  const RAYON_POINT = { plein: 1.25, accent: 1.4, trame: 1.05 };

  // Mouvement : ressort vers la place, amorti ; poussée de la souris.
  const RESSORT = 0.0075;
  const AMORTI = 0.9;
  const PORTEE = 95;
  const POUSSEE = 1.7;

  // ----- Outils de dessin, dans la boîte de 100 -----

  function rect(c, x, y, l, h) {
    c.fillRect(x, y, l, h);
  }

  function chemin_arche(c, x, y, l, h) {
    const r = l / 2;
    c.moveTo(x, y + h);
    c.lineTo(x, y + r);
    c.arc(x + r, y + r, r, Math.PI, 0);
    c.lineTo(x + l, y + h);
    c.closePath();
  }

  function arche(c, x, y, l, h) {
    c.beginPath();
    chemin_arche(c, x, y, l, h);
    c.fill();
  }

  function trait_arche(c, x, y, l, h, epaisseur) {
    c.beginPath();
    chemin_arche(c, x, y, l, h);
    c.lineWidth = epaisseur;
    c.stroke();
  }

  function disque(c, x, y, r) {
    c.beginPath();
    c.arc(x, y, r, 0, Math.PI * 2);
    c.fill();
  }

  function polygone(c, points) {
    c.beginPath();
    c.moveTo(points[0][0], points[0][1]);
    for (let i = 1; i < points.length; i += 1) {
      c.lineTo(points[i][0], points[i][1]);
    }
    c.closePath();
  }

  function creuser(c, dessin) {
    c.save();
    c.globalCompositeOperation = "destination-out";
    dessin();
    c.restore();
  }

  // Colonnade : fûts cannelés, chapiteaux.
  function colonnes(c, n, x0, x1, haut, bas, largeur) {
    const pas = (x1 - x0 - largeur) / (n - 1);
    for (let i = 0; i < n; i += 1) {
      const x = x0 + i * pas;
      rect(c, x, haut + 2.4, largeur, bas - haut - 2.4);
      rect(c, x - 0.9, haut, largeur + 1.8, 2.4);
      creuser(c, function () {
        rect(c, x + largeur * 0.33, haut + 4, 0.55, bas - haut - 5);
        rect(c, x + largeur * 0.62, haut + 4, 0.55, bas - haut - 5);
      });
    }
  }

  // ----- Figures -----

  const FIGURES = {
    portails: {
      nom: "Portails",
      texte: "L'arche du portail, motif de l'identité.",
      plein(c) {
        trait_arche(c, 12, 32, 22, 54, 1.7);
        trait_arche(c, 39, 18, 22, 68, 1.7);
        trait_arche(c, 66, 32, 22, 54, 1.7);
        rect(c, 4, 86, 92, 1.4);
      },
      accent(c) {
        disque(c, 50, 42, 7.5);
      },
      trame(c) {
        arche(c, 12, 32, 22, 54);
        arche(c, 39, 18, 22, 68);
        arche(c, 66, 32, 22, 54);
      },
    },

    parthenon: {
      nom: "Parthénon",
      texte: "Athènes, Ve siècle avant notre ère : huit colonnes doriques en façade.",
      plein(c) {
        rect(c, 6, 83, 88, 3);
        rect(c, 8, 80, 84, 3);
        rect(c, 10, 77, 80, 3);
        colonnes(c, 8, 13, 87, 44, 77, 5.4);
        rect(c, 10, 37.5, 80, 5);
        rect(c, 10, 31.5, 80, 5);
        creuser(c, function () {
          for (let x = 12.5; x < 88; x += 5) {
            rect(c, x, 32.3, 2.6, 3.4);
          }
        });
        rect(c, 8, 29.5, 84, 1.6);
        c.beginPath();
        c.moveTo(9, 29);
        c.lineTo(50, 13);
        c.lineTo(91, 29);
        c.closePath();
        c.lineWidth = 1.6;
        c.stroke();
      },
      accent(c) {
        disque(c, 80, 15, 8);
      },
      trame(c) {
        polygone(c, [[9, 29], [50, 13], [91, 29]]);
        c.fill();
        rect(c, 14, 44, 72, 33);
      },
    },

    colisee: {
      nom: "Colisée",
      texte: "Rome, Ier siècle : trois rangs d'arcades sous un attique, l'aile en ruine.",
      contour: [[5, 87], [5, 15], [58, 15], [62, 21], [69, 21], [73, 29], [81, 29], [85, 40], [95, 40], [95, 87]],
      plein(c) {
        polygone(c, this.contour);
        c.fill();
        creuser(c, function () {
          for (let x = 8; x < 92; x += 9.4) {
            arche(c, x, 64, 6.4, 21);
            arche(c, x, 44, 6.4, 16);
            arche(c, x, 28, 6.4, 12);
          }
          for (let x = 10.2; x < 92; x += 18.8) {
            rect(c, x, 18, 2.4, 3.2);
          }
          rect(c, 5, 61.6, 90, 0.7);
          rect(c, 5, 41.6, 90, 0.7);
          rect(c, 5, 25.6, 90, 0.7);
        });
        rect(c, 2, 87, 96, 1.4);
      },
      accent(c) {
        disque(c, 24, 12, 9);
      },
      trame(c) {
        polygone(c, this.contour);
        c.fill();
      },
    },

    pantheon: {
      nom: "Panthéon",
      texte: "Rome, IIe siècle : la coupole sur son tambour, derrière le portique.",
      plein(c) {
        c.beginPath();
        c.arc(50, 42, 25, Math.PI, 0);
        c.lineWidth = 1.6;
        c.stroke();
        for (const h of [36, 30, 24]) {
          const demi = Math.sqrt(25 * 25 - (42 - h) * (42 - h));
          rect(c, 50 - demi, h, demi * 2, 0.7);
        }
        rect(c, 46.5, 16, 7, 1.4);
        c.strokeRect(25.8, 42, 48.4, 12);
        rect(c, 12, 85, 76, 2.6);
        rect(c, 14, 82.4, 72, 2.6);
        colonnes(c, 8, 17, 83, 57, 82.4, 4.4);
        rect(c, 15, 52.5, 70, 4.5);
        c.beginPath();
        c.moveTo(13, 52.5);
        c.lineTo(50, 40);
        c.lineTo(87, 52.5);
        c.closePath();
        c.lineWidth = 1.6;
        c.stroke();
      },
      accent(c) {
        disque(c, 83, 18, 6.5);
      },
      trame(c) {
        c.beginPath();
        c.arc(50, 42, 25, Math.PI, 0);
        c.fill();
        rect(c, 25, 42, 50, 12);
        polygone(c, [[13, 52.5], [50, 40], [87, 52.5]]);
        c.fill();
        rect(c, 17, 57, 66, 25.4);
      },
    },

    aqueduc: {
      nom: "Aqueduc",
      texte: "Le pont du Gard, Ier siècle : trois rangs d'arches sur la rivière.",
      plein(c) {
        rect(c, 3, 61, 94, 27);
        rect(c, 3, 37, 94, 22);
        rect(c, 3, 25, 94, 10);
        rect(c, 3, 22, 94, 2);
        creuser(c, function () {
          for (let i = 0; i < 5; i += 1) {
            const centre = 12.4 + i * 18.8;
            arche(c, centre - 6.5, 65, 13, 23);
            arche(c, centre - 6.5, 40, 13, 19);
          }
          for (let x = 4.6; x < 95; x += 4.7) {
            arche(c, x, 27, 3.1, 8);
          }
        });
      },
      accent(c) {
        disque(c, 50, 79, 5.5);
      },
      trame(c) {
        rect(c, 3, 22, 94, 66);
        rect(c, 0, 90, 100, 6);
      },
    },

    chouette: {
      nom: "Chouette d'Athéna",
      texte: "La chouette des tétradrachmes d'Athènes, avec l'olivier et les lettres ΑΘΕ.",
      plein(c) {
        c.beginPath();
        c.arc(50, 50, 44, 0, Math.PI * 2);
        c.lineWidth = 1.3;
        c.stroke();
        c.beginPath();
        c.ellipse(52, 61, 14, 20, 0, 0, Math.PI * 2);
        c.fill();
        disque(c, 52, 35, 11.5);
        polygone(c, [[42, 28], [44, 19.5], [48, 25.5]]);
        c.fill();
        polygone(c, [[62, 28], [60, 19.5], [56, 25.5]]);
        c.fill();
        rect(c, 46.5, 79.5, 3, 4);
        rect(c, 54.5, 79.5, 3, 4);
        rect(c, 32, 83.5, 40, 1.2);
        creuser(c, function () {
          disque(c, 47.5, 34, 4.4);
          disque(c, 56.5, 34, 4.4);
          for (let i = 0; i < 4; i += 1) {
            c.beginPath();
            c.ellipse(58, 56 + i * 5.5, 9, 3, -0.35, 0.2, 2.2);
            c.lineWidth = 0.7;
            c.stroke();
          }
        });
        c.beginPath();
        c.moveTo(20, 34);
        c.quadraticCurveTo(26, 24, 35, 20);
        c.lineWidth = 0.9;
        c.stroke();
        for (const [x, y, a] of [[22.5, 27, -1.1], [27, 22, -0.5], [31.5, 24.5, 0.3], [25, 31, -0.2]]) {
          c.beginPath();
          c.ellipse(x, y, 3.6, 1.4, a, 0, Math.PI * 2);
          c.fill();
        }
        c.font = "600 9px Georgia, 'Times New Roman', serif";
        c.fillText("ΑΘΕ", 69, 66);
      },
      accent(c) {
        disque(c, 47.5, 34, 3.4);
        disque(c, 56.5, 34, 3.4);
        creuser(c, function () {
          disque(c, 47.5, 34, 1.2);
          disque(c, 56.5, 34, 1.2);
        });
      },
      trame(c) {
        disque(c, 50, 50, 41);
      },
    },

    amphore: {
      nom: "Amphore",
      texte: "Amphore attique : panse, anses et frise de méandres.",
      corps(c) {
        c.beginPath();
        c.moveTo(44, 14);
        c.bezierCurveTo(44, 23, 28, 28, 28, 45);
        c.bezierCurveTo(28, 62, 42, 74, 45, 82);
        c.lineTo(55, 82);
        c.bezierCurveTo(58, 74, 72, 62, 72, 45);
        c.bezierCurveTo(72, 28, 56, 23, 56, 14);
        c.closePath();
      },
      plein(c) {
        this.corps(c);
        c.fill();
        rect(c, 40, 10.5, 20, 3.5);
        rect(c, 39, 82, 22, 4);
        rect(c, 34, 87.5, 32, 1.2);
        c.lineWidth = 2;
        c.beginPath();
        c.moveTo(44, 18);
        c.bezierCurveTo(33, 15, 28, 23, 31.5, 35);
        c.stroke();
        c.beginPath();
        c.moveTo(56, 18);
        c.bezierCurveTo(67, 15, 72, 23, 68.5, 35);
        c.stroke();
        creuser(c, function () {
          rect(c, 20, 36.5, 60, 17.5);
          rect(c, 20, 57, 60, 7);
        });
      },
      accent(c) {
        c.save();
        this.corps(c);
        c.clip();
        c.lineWidth = 1;
        c.beginPath();
        for (let x = 26; x < 74; x += 6) {
          c.moveTo(x, 63.5);
          c.lineTo(x, 57.8);
          c.lineTo(x + 4.6, 57.8);
          c.lineTo(x + 4.6, 61.6);
          c.lineTo(x + 2.2, 61.6);
          c.lineTo(x + 2.2, 59.8);
        }
        c.stroke();
        rect(c, 20, 63.5, 60, 0.9);
        c.restore();
      },
      trame(c) {
        this.corps(c);
        c.fill();
      },
    },
  };

  const ORDRE = ["portails", "parthenon", "colisee", "pantheon", "aqueduc", "chouette", "amphore"];

  // ----- Échantillonnage d'une figure en points -----

  let largeur = 0;
  let hauteur = 0;
  let ratio = 1;
  let figure = ORDRE.indexOf(window.location.hash.slice(1)) >= 0 ? window.location.hash.slice(1) : "portails";
  let particules = [];
  const souris = { x: -1e5, y: -1e5 };

  function calque(nom, dessin) {
    const toile_hors = document.createElement("canvas");
    toile_hors.width = largeur;
    toile_hors.height = hauteur;
    const c = toile_hors.getContext("2d");
    const cote = Math.min(largeur, hauteur) * 0.84;
    const echelle = cote / 100;
    c.setTransform(echelle, 0, 0, echelle, (largeur - cote) / 2, (hauteur - cote) / 2 - hauteur * 0.03);
    c.fillStyle = "#000";
    c.strokeStyle = "#000";
    if (dessin) {
      dessin.call(FIGURES[figure], c);
    }
    return c.getImageData(0, 0, largeur, hauteur).data;
  }

  function cibles() {
    const f = FIGURES[figure];
    const plein = calque("plein", f.plein);
    const accent = calque("accent", f.accent);
    const trame = calque("trame", f.trame);
    const opaque = function (donnees, x, y) {
      const xi = Math.round(x);
      const yi = Math.round(y);
      if (xi < 0 || yi < 0 || xi >= largeur || yi >= hauteur) {
        return false;
      }
      return donnees[(yi * largeur + xi) * 4 + 3] > 100;
    };
    const liste = [];
    const semer = function (nom, donnees, masques) {
      const pas = PAS[nom];
      for (let y = pas / 2; y < hauteur; y += pas) {
        for (let x = pas / 2; x < largeur; x += pas) {
          const jx = x + (Math.random() - 0.5) * pas * 0.35;
          const jy = y + (Math.random() - 0.5) * pas * 0.35;
          if (!opaque(donnees, jx, jy)) {
            continue;
          }
          if (masques.some(function (m) { return opaque(m, jx, jy); })) {
            continue;
          }
          liste.push({ x: jx, y: jy, nom: nom });
        }
      }
    };
    semer("plein", plein, []);
    semer("accent", accent, [plein]);
    semer("trame", trame, [plein, accent]);
    for (let i = liste.length - 1; i > 0; i -= 1) {
      const j = Math.floor(Math.random() * (i + 1));
      const t = liste[i];
      liste[i] = liste[j];
      liste[j] = t;
    }
    return liste;
  }

  function placer(depuis_le_bord) {
    const liste = cibles();
    const garde = [];
    for (let i = 0; i < liste.length; i += 1) {
      const cible = liste[i];
      let p = particules[i];
      if (!p || p.mourante) {
        p = depuis_le_bord
          ? { x: Math.random() * largeur, y: Math.random() * hauteur, vx: 0, vy: 0 }
          : { x: cible.x, y: cible.y, vx: 0, vy: 0 };
      }
      p.hx = cible.x;
      p.hy = cible.y;
      p.nom = cible.nom;
      p.alpha = 1;
      p.mourante = false;
      garde.push(p);
    }
    // Points en trop : ils s'éloignent en s'effaçant.
    for (let i = liste.length; i < particules.length; i += 1) {
      const p = particules[i];
      if (p.mourante) {
        continue;
      }
      p.mourante = true;
      const angle = Math.random() * Math.PI * 2;
      p.hx = p.x + Math.cos(angle) * 60;
      p.hy = p.y + Math.sin(angle) * 60;
      garde.push(p);
    }
    particules = garde;
    if (capture || reduit) {
      for (const p of particules) {
        p.x = p.hx;
        p.y = p.hy;
        p.vx = 0;
        p.vy = 0;
      }
    }
    if (legende) {
      legende.querySelector("strong").textContent = FIGURES[figure].nom;
      legende.querySelector("span").textContent = FIGURES[figure].texte;
    }
  }

  function dimensionner() {
    const boite = panneau.getBoundingClientRect();
    const l = Math.max(1, Math.round(boite.width));
    const h = Math.max(1, Math.round(boite.height));
    if (l === largeur && h === hauteur) {
      return;
    }
    const premier = largeur === 0;
    largeur = l;
    hauteur = h;
    ratio = window.devicePixelRatio || 1;
    toile.width = Math.round(largeur * ratio);
    toile.height = Math.round(hauteur * ratio);
    placer(premier && !capture && !reduit);
  }

  // ----- Animation -----

  function image() {
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, largeur, hauteur);
    const portee2 = PORTEE * PORTEE;
    const parCouleur = { plein: [], accent: [], trame: [], efface: [] };
    const vivantes = [];
    for (const p of particules) {
      let ax = (p.hx - p.x) * RESSORT;
      let ay = (p.hy - p.y) * RESSORT;
      if (!reduit) {
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
      if (reduit) {
        p.x = p.hx;
        p.y = p.hy;
      } else {
        p.x += p.vx;
        p.y += p.vy;
      }
      if (p.mourante) {
        p.alpha -= 0.03;
        if (p.alpha <= 0) {
          continue;
        }
        parCouleur.efface.push(p);
      } else {
        parCouleur[p.nom].push(p);
      }
      vivantes.push(p);
    }
    particules = vivantes;
    for (const nom of ["trame", "plein", "accent"]) {
      ctx.fillStyle = COULEURS[nom];
      ctx.beginPath();
      const r = RAYON_POINT[nom];
      for (const p of parCouleur[nom]) {
        ctx.moveTo(p.x + r, p.y);
        ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      }
      ctx.fill();
    }
    for (const p of parCouleur.efface) {
      ctx.globalAlpha = Math.max(0, p.alpha) * 0.6;
      ctx.fillStyle = COULEURS.trame;
      ctx.fillRect(p.x - 1, p.y - 1, 2, 2);
    }
    ctx.globalAlpha = 1;
    window.requestAnimationFrame(image);
  }

  // ----- Souris -----

  const figee = parametres.get("souris");
  if (figee) {
    const [fx, fy] = figee.split(",").map(Number);
    const fixer = function () {
      souris.x = fx * largeur;
      souris.y = fy * hauteur;
    };
    window.addEventListener("resize", fixer);
    window.setTimeout(fixer, 0);
  } else {
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

  // ----- Choix de la figure (maquette) -----

  const boutons = document.querySelectorAll(".choix-figure button");
  function marquer() {
    boutons.forEach(function (b) {
      b.setAttribute("aria-pressed", b.dataset.figure === figure ? "true" : "false");
    });
  }
  boutons.forEach(function (b) {
    b.addEventListener("click", function () {
      figure = b.dataset.figure;
      window.history.replaceState(null, "", "#" + figure);
      marquer();
      placer(false);
    });
  });
  marquer();

  dimensionner();
  if (window.ResizeObserver) {
    new ResizeObserver(dimensionner).observe(panneau);
  } else {
    window.addEventListener("resize", dimensionner);
  }
  window.requestAnimationFrame(image);
})();
