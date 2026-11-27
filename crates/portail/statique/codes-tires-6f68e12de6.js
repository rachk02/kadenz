// Portail des licences : bouton « Copier » de chaque code tiré (demande de
// l'utilisateur du 2026-11-27). Servi par le portail (CSP script-src 'self'),
// sans eval ni script en ligne. Sans lui, les boutons restent cachés et les
// codes se sélectionnent à la main ; le téléchargement de la liste ne dépend
// pas de ce script.

(function () {
  "use strict";

  const avis = document.getElementById("avis-copie");

  function annoncer(texte) {
    if (avis) {
      avis.textContent = "";
      avis.textContent = texte;
    }
  }

  // Repli pour un navigateur sans presse-papiers asynchrone.
  function copierParSelection(texte) {
    const zone = document.createElement("textarea");
    zone.value = texte;
    zone.setAttribute("readonly", "");
    zone.className = "avis-code";
    document.body.appendChild(zone);
    zone.select();
    let reussi = false;
    try {
      reussi = document.execCommand("copy");
    } catch (e) {
      reussi = false;
    }
    document.body.removeChild(zone);
    return reussi ? Promise.resolve() : Promise.reject(new Error("copie refusée"));
  }

  function copier(texte) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(texte).catch(function () {
        return copierParSelection(texte);
      });
    }
    return copierParSelection(texte);
  }

  document.querySelectorAll("button.bouton-copier[data-code]").forEach(function (bouton) {
    let minuterie = null;
    bouton.hidden = false;
    bouton.addEventListener("click", function () {
      const code = bouton.getAttribute("data-code");
      copier(code).then(
        function () {
          bouton.classList.add("copie");
          bouton.setAttribute("aria-label", "Code copié");
          annoncer("Code " + code + " copié.");
          clearTimeout(minuterie);
          minuterie = setTimeout(function () {
            bouton.classList.remove("copie");
            bouton.setAttribute("aria-label", "Copier le code " + code);
          }, 1600);
        },
        function () {
          annoncer("Copie impossible : sélectionnez le code à la main.");
        }
      );
    });
  });
})();
