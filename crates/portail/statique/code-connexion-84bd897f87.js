// Portail des licences : code de connexion en six cases (choix de
// l'utilisateur du 2026-11-19). Seul script du portail, servi par lui-même
// (CSP script-src 'self'), sans eval ni script en ligne. Il ne fait
// qu'embellir : sans lui, la page garde son champ unique et son bouton, et
// marche à l'identique.
//
// Six chiffres saisis : le formulaire part au serveur, qui seul vérifie le
// code. Réponse en redirection (seule réponse d'un code accepté) : cases
// vertes, avis, puis ouverture du portail. Toute autre réponse : cases
// rouges et message du serveur ; l'étape encore valable, les cases se vident,
// sinon retour à la connexion. Les codes de secours passent par le champ
// unique, rendu par « Utiliser un code de secours ».

(function () {
  "use strict";

  const formulaire = document.querySelector('form[action="/connexion/code"]');
  const champ = document.getElementById("code");
  if (!formulaire || !champ || !window.fetch || !window.DOMParser) {
    return;
  }
  const blocChamp = champ.closest(".champ");
  const bouton = formulaire.querySelector('button[type="submit"]');
  const NB = 6;

  // ----- Construction des cases -----

  const groupe = document.createElement("fieldset");
  groupe.className = "chiffres-code";
  const legende = document.createElement("legend");
  legende.textContent = "Code à 6 chiffres";
  groupe.appendChild(legende);
  const rangee = document.createElement("div");
  rangee.className = "chiffres";
  const cases = [];
  for (let i = 0; i < NB; i += 1) {
    if (i === NB / 2) {
      const tiret = document.createElement("span");
      tiret.className = "chiffres-tiret";
      tiret.setAttribute("aria-hidden", "true");
      rangee.appendChild(tiret);
    }
    const c = document.createElement("input");
    c.type = "text";
    c.inputMode = "numeric";
    // Pas de longueur maximale sur la première case : un gestionnaire de
    // mots de passe y colle le code entier, réparti ensuite sur les six.
    if (i > 0) {
      c.maxLength = 1;
    }
    c.autocomplete = i === 0 ? "one-time-code" : "off";
    c.setAttribute("aria-label", "Chiffre " + (i + 1));
    rangee.appendChild(c);
    cases.push(c);
  }
  groupe.appendChild(rangee);

  const avis = document.createElement("p");
  avis.className = "avis-code";
  avis.setAttribute("role", "status");
  avis.setAttribute("aria-live", "polite");
  avis.hidden = true;

  const bascule = document.createElement("p");
  bascule.className = "bascule-code";
  const lien = document.createElement("button");
  lien.type = "button";
  lien.className = "lien-code";
  lien.textContent = "Utiliser un code de secours";
  bascule.appendChild(lien);

  formulaire.insertBefore(groupe, blocChamp);
  formulaire.appendChild(avis);
  formulaire.appendChild(bascule);
  blocChamp.hidden = true;
  bouton.hidden = true;
  champ.required = false;
  champ.autofocus = false;
  cases[0].focus();

  // ----- Avis -----

  function montrer(etat, texte) {
    avis.className = "avis-code avis-code-" + etat;
    avis.textContent = texte;
    avis.hidden = false;
  }

  function cacher() {
    avis.hidden = true;
  }

  // ----- Envoi -----

  let occupe = false;

  function vider() {
    groupe.classList.remove("faux");
    cases.forEach(function (c) {
      c.value = "";
      c.readOnly = false;
    });
    occupe = false;
    cases[0].focus();
  }

  function envoyer() {
    const saisi = cases.map(function (c) { return c.value; }).join("");
    if (occupe || saisi.length !== NB) {
      return;
    }
    occupe = true;
    champ.value = saisi;
    cases.forEach(function (c) { c.readOnly = true; });
    groupe.classList.add("verification");
    cacher();
    window
      .fetch(formulaire.action, {
        method: "POST",
        body: new URLSearchParams(new FormData(formulaire)),
        credentials: "same-origin",
        redirect: "manual",
      })
      .then(function (reponse) {
        groupe.classList.remove("verification");
        if (reponse.type === "opaqueredirect") {
          groupe.classList.add("juste");
          montrer("ok", "Code accepté : ouverture du portail.");
          window.setTimeout(function () {
            window.location.assign("/");
          }, 900);
          return null;
        }
        return reponse.text();
      })
      .then(function (texte) {
        if (texte === null) {
          return;
        }
        const page = new DOMParser().parseFromString(texte, "text/html");
        const message = page.querySelector(".message-erreur");
        const encore = page.querySelector('form[action="/connexion/code"]');
        const etapeRecue = encore && encore.querySelector('input[name="etape"]');
        const etapeIci = formulaire.querySelector('input[name="etape"]');
        if (etapeRecue && etapeIci) {
          etapeIci.value = etapeRecue.value;
        }
        groupe.classList.add("faux");
        montrer("erreur", message ? message.textContent.trim() : "Code refusé.");
        window.setTimeout(function () {
          if (encore) {
            vider();
          } else {
            // Étape close (délai, trop de codes faux) : retour à la connexion.
            window.location.assign("/connexion");
          }
        }, encore ? 1300 : 2200);
      })
      .catch(function () {
        // Réseau coupé : envoi ordinaire du formulaire, le serveur répond.
        formulaire.submit();
      });
  }

  // ----- Saisie -----

  function remplir(depuis, texte) {
    const chiffres = texte.replace(/\D/g, "").split("");
    let i = depuis;
    for (const ch of chiffres) {
      if (i >= NB) {
        break;
      }
      cases[i].value = ch;
      i += 1;
    }
    cases[Math.min(i, NB - 1)].focus();
    envoyer();
  }

  cases.forEach(function (c, i) {
    c.addEventListener("input", function () {
      if (occupe) {
        return;
      }
      if (avis.classList.contains("avis-code-erreur")) {
        cacher();
      }
      const chiffres = c.value.replace(/\D/g, "");
      if (chiffres.length > 1) {
        c.value = "";
        remplir(i, chiffres);
        return;
      }
      c.value = chiffres;
      if (chiffres && i < NB - 1) {
        cases[i + 1].focus();
      }
      envoyer();
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
      } else if (e.key === "ArrowRight" && i < NB - 1) {
        e.preventDefault();
        cases[i + 1].focus();
      }
    });
    c.addEventListener("paste", function (e) {
      e.preventDefault();
      if (!occupe) {
        remplir(i, (e.clipboardData || window.clipboardData).getData("text"));
      }
    });
    c.addEventListener("focus", function () {
      c.select();
    });
  });

  // Entrée dans une case : envoi par le script, jamais un code incomplet.
  formulaire.addEventListener("submit", function (e) {
    if (!groupe.hidden) {
      e.preventDefault();
      envoyer();
    }
  });

  // ----- Code de secours : retour au champ unique -----

  lien.addEventListener("click", function () {
    const versSecours = blocChamp.hidden;
    blocChamp.hidden = !versSecours;
    bouton.hidden = !versSecours;
    groupe.hidden = versSecours;
    champ.required = versSecours;
    champ.value = "";
    lien.textContent = versSecours ? "Utiliser le code de l'application" : "Utiliser un code de secours";
    cacher();
    (versSecours ? champ : cases[0]).focus();
  });
})();
