//! Enregistrement d'un paiement reçu (GET et POST
//! /entreprises/{identifiant}/paiement), décision 60 : montant, devise et
//! nombre de postes saisis par l'opérateur, aucune grille de prix.

use maud::{Markup, html};

use super::{Cadre, Rubrique, chemin_entreprise, erreur_formulaire, jeton, page, titre};
use crate::{Entreprise, Mode};

pub fn nouveau_paiement(c: &Cadre, e: &Entreprise, erreur: Option<&str>) -> Markup {
    let chemin = chemin_entreprise(&e.identifiant);
    let contenu = html! {
        (titre("Enregistrer un paiement", html! {
            a.bouton href=(chemin) { "Retour à l'entreprise" }
        }))
        p.texte { "Client : " strong { (e.client) } ". Le paiement enregistré sert ensuite à émettre une seule licence." }
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action={ (chemin) "/paiement" } {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.rangee {
                        div.champ {
                            label for="recu_le" { "Reçu le" }
                            input #recu_le type="date" name="recu_le" required;
                        }
                        div.champ {
                            label for="mode" { "Mode de paiement" }
                            select #mode name="mode" required {
                                @for m in Mode::TOUS {
                                    option value=(m.code()) { (m.libelle()) }
                                }
                            }
                        }
                    }
                    div.rangee {
                        div.champ {
                            label for="montant" { "Montant reçu" }
                            input #montant type="number" name="montant" min="1" step="1"
                                inputmode="numeric" required;
                        }
                        div.champ {
                            label for="devise" { "Devise" }
                            input #devise type="text" name="devise" value="XOF" required
                                minlength="3" maxlength="3" autocomplete="off" spellcheck="false";
                            p.aide { "Code à trois lettres : XOF pour le franc CFA d'Afrique de l'Ouest." }
                        }
                    }
                    div.champ {
                        label for="reference" { "Référence (facultative)" }
                        input #reference type="text" name="reference" autocomplete="off";
                        p.aide { "Numéro du chèque ou référence du virement." }
                    }
                    div.rangee {
                        div.champ {
                            label for="postes" { "Nombre de postes payés" }
                            input #postes type="number" name="postes" min="1" step="1"
                                inputmode="numeric" required;
                        }
                        div.champ {
                            label for="formule" { "Formule" }
                            select #formule name="formule" required {
                                option value="mensuelle" { "Mensuelle" }
                                option value="annuelle" { "Annuelle" }
                            }
                        }
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Enregistrer le paiement" }
                        a.bouton href=(chemin) { "Annuler" }
                    }
                }
            }
        }
    };
    page(c, "Enregistrer un paiement", Rubrique::Entreprises, contenu)
}
