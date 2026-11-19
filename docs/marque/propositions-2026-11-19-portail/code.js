// Portail des licences : saisie du code de connexion en six cases (maquette
// du 2026-11-19, kadenz-e6). Un chiffre par case, passage automatique à la
// suivante, collage d'un code entier, retour arrière vers la précédente.
// Six chiffres saisis : validation sans bouton ; juste, les cases passent au
// vert, un avis s'affiche et la page suivante s'ouvre ; faux, les cases
// passent au rouge, l'avis donne le message du portail, puis tout se vide.
//
// Maquette : le code juste est celui de `data-exemple` sur le formulaire.
// Dans le portail, le code partirait au serveur, qui seul le vérifie.

(function () {
  "use strict";

  const formulaire = document.querySelector(".formulaire-code");
  if (!formulaire) {
    return;
  }
  const groupe = document.getElementById("cases-code");
  const cases = Array.from(groupe.querySelectorAll("input"));
  const avis = document.getElementById("avis");
  const avisTexte = document.getElementById("avis-texte");
  const secours = document.getElementById("secours");
  const bascule = document.getElementById("bascule-secours");
  const exemple = formulaire.dataset.exemple;
  const suite = formulaire.dataset.suite;
  let occupe = false;
  let minuterieAvis = 0;

  function montrerAvis(etat, texte) {
    window.clearTimeout(minuterieAvis);
    avis.hidden = false;
    avis.className = "avis avis-" + etat;
    avisTexte.textContent = texte;
    // Relancer l'apparition à chaque avis.
    void avis.offsetWidth;
    avis.classList.add("visible");
  }

  function cacherAvis() {
    avis.classList.remove("visible");
    minuterieAvis = window.setTimeout(function () {
      avis.hidden = true;
    }, 250);
  }

  function code() {
    return cases.map(function (c) { return c.value; }).join("");
  }

  function valider() {
    const saisi = code();
    if (saisi.length !== cases.length || occupe) {
      return;
    }
    occupe = true;
    cases.forEach(function (c) { c.readOnly = true; });
    groupe.classList.add("verification");
    // Le temps d'une vérification par le serveur.
    window.setTimeout(function () {
      groupe.classList.remove("verification");
      if (saisi === exemple) {
        groupe.classList.add("juste");
        montrerAvis("ok", "Code accepté : ouverture du portail.");
        window.setTimeout(function () {
          window.location.href = suite;
        }, 1100);
        return;
      }
      groupe.classList.add("faux");
      montrerAvis("erreur", "Code incorrect ou déjà utilisé.");
      window.setTimeout(function () {
        groupe.classList.remove("faux");
        cases.forEach(function (c) {
          c.value = "";
          c.readOnly = false;
        });
        occupe = false;
        cases[0].focus();
      }, 1300);
    }, 450);
  }

  function remplirDepuis(index, texte) {
    const chiffres = texte.replace(/\D/g, "").split("");
    let i = index;
    for (const ch of chiffres) {
      if (i >= cases.length) {
        break;
      }
      cases[i].value = ch;
      i += 1;
    }
    cases[Math.min(i, cases.length - 1)].focus();
    valider();
  }

  cases.forEach(function (c, i) {
    c.addEventListener("input", function () {
      if (occupe) {
        return;
      }
      if (avis.classList.contains("avis-erreur")) {
        cacherAvis();
      }
      const chiffres = c.value.replace(/\D/g, "");
      if (chiffres.length > 1) {
        c.value = "";
        remplirDepuis(i, chiffres);
        return;
      }
      c.value = chiffres;
      if (chiffres && i < cases.length - 1) {
        cases[i + 1].focus();
      }
      valider();
    });

    c.addEventListener("keydown", function (e) {
      if (occupe) {
        return;
      }
      if (e.key === "Backspace" && !c.value && i > 0) {
        e.preventDefault();
        cases[i - 1].value = "";
        cases[i - 1].focus();
      } else if (e.key === "ArrowLeft" && i > 0) {
        e.preventDefault();
        cases[i - 1].focus();
      } else if (e.key === "ArrowRight" && i < cases.length - 1) {
        e.preventDefault();
        cases[i + 1].focus();
      }
    });

    c.addEventListener("paste", function (e) {
      e.preventDefault();
      if (occupe) {
        return;
      }
      remplirDepuis(i, (e.clipboardData || window.clipboardData).getData("text"));
    });

    c.addEventListener("focus", function () {
      c.select();
    });
  });

  bascule.addEventListener("click", function () {
    const versSecours = secours.hidden;
    secours.hidden = !versSecours;
    groupe.hidden = versSecours;
    bascule.textContent = versSecours ? "Utiliser le code de l'application" : "Utiliser un code de secours";
    cacherAvis();
    (versSecours ? document.getElementById("code-secours") : cases[0]).focus();
  });

  formulaire.addEventListener("submit", function (e) {
    // Maquette : un code de secours n'est pas vérifié ici.
    if (!secours.hidden) {
      e.preventDefault();
      montrerAvis("erreur", "Code incorrect ou déjà utilisé.");
    } else {
      e.preventDefault();
      valider();
    }
  });
})();
