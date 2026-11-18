"""Motifs animés adoptés le 2026-11-18, pour l'application (Slint) : A2 Trame de pièces
(présentation, connexion), A+ Anneaux (inscription du poste), C Tournant (colonne des
étapes de l'assistant). Même géométrie que la page de maquettes (generer.py, index.html).

Écrit :
- crates/app/ui/icones/motifs/*.svg : pièces seules, fonds ton sur ton, couches du reflet,
  couches des anneaux (dessin blanc recoloré par Slint, sauf les anneaux, déjà en couleur) ;
- crates/app/ui/motifs-ecrans.slint : couleurs, données des cases et composants
  MotifA2, MotifAnneaux, MotifTournant.

Usage : py -3.13 generer_motifs_slint.py
"""
import math
from pathlib import Path

ICI = Path(__file__).resolve().parent
UI = ICI.parents[2] / "crates" / "app" / "ui"
SORTIE = UI / "icones" / "motifs"

PIECES = {
    "carre": "M16 16 L120 16 L120 120 L16 120 Z",
    "quart": "M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z",
    "tri": "M16 136 L120 240 L16 240 Z",
    "disque": "M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z",
}
ORDRE = ["carre", "quart", "tri", "disque"]
CENTRE = {"carre": (68, 68), "quart": (188, 68), "tri": (68, 188), "disque": (188, 188)}
QUART_TOURNANT = "M0 104 L0 4.16 A99.84 99.84 0 0 1 99.84 104 Z"

# Panneau de référence : 640 x 760, posé en « cover » dans le panneau réel.
W, H = 640, 760
CELL = W / 14
TAILLE = CELL * 0.86


def dans(k, u, v):
    if k == "carre":
        return 16 <= u <= 120 and 16 <= v <= 120
    if k == "quart":
        return u >= 136 and v <= 120 and (u - 136) ** 2 + (v - 120) ** 2 <= 105.56 ** 2
    if k == "tri":
        return u >= 16 and 136 <= v <= 240 and (u - 16) <= (v - 136)
    return (u - 188) ** 2 + (v - 188) ** 2 <= 53.56 ** 2


def placement(piece):
    cell = 0.6 * min(W, H * 0.8)
    cx, cy = W / 2, H * 0.42
    S = cell * 256 / 104
    px, py = CENTRE[piece]
    return cell, S, cx - px / 256 * S, cy - py / 256 * S


def nette(x, y):
    return y > H - 150 or y < 100


def f(x):
    return f"{x:.2f}".rstrip("0").rstrip(".")


def defs():
    return "<defs>" + "".join(f'<path id="{k}" d="{d}"/>' for k, d in PIECES.items()) + "</defs>"


def use(k, cx, cy, s):
    px, py = CENTRE[k]
    sc = s / 104
    return f'<use href="#{k}" transform="translate({f(cx - px * sc)} {f(cy - py * sc)}) scale({f(sc)})"/>'


def svg(w, h, corps, vb=None):
    vb = vb or f"0 0 {f(w)} {f(h)}"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
            f'width="{f(w)}" height="{f(h)}" viewBox="{vb}" aria-hidden="true">{corps}</svg>')


def a2(piece):
    """Cases ambre (données Slint), fond ton sur ton et couches du reflet (SVG)."""
    _, S, ox, oy = placement(piece)
    pcx, pcy = CENTRE[piece]
    ambre, fond, reflet = [], [], [[] for _ in range(7)]
    for j in range(math.ceil(H / CELL)):
        for i in range(14):
            cx, cy = i * CELL + CELL / 2, j * CELL + CELL / 2
            kk = ORDRE[(i + j) % 4]
            u, v = (cx - ox) / S * 256, (cy - oy) / S * 256
            if dans(piece, u, v):
                r = min(1, max(0, ((u - pcx) + (v - pcy)) / 208 + 0.5))
                ambre.append((cx, cy, ORDRE.index(kk), r))
                continue
            if nette(cx, cy):
                continue
            fond.append(use(kk, cx, cy, TAILLE))
            reflet[((i + j) % 14) // 2].append(use(kk, cx, cy, TAILLE))
    return ambre, svg(W, H, defs() + '<g fill="#fff">' + "".join(fond) + "</g>"), \
        [svg(W, H, defs() + '<g fill="#fff">' + "".join(c) + "</g>") for c in reflet]


def alea(graine):
    a = graine & 0xFFFFFFFF

    def suivant():
        nonlocal a
        a = (a + 0x6D2B79F5) & 0xFFFFFFFF
        t = a
        t = ((t ^ (t >> 15)) * (t | 1)) & 0xFFFFFFFF
        t ^= (t + (((t ^ (t >> 7)) * (t | 61)) & 0xFFFFFFFF)) & 0xFFFFFFFF
        return ((t ^ (t >> 14)) & 0xFFFFFFFF) / 4294967296
    return suivant


# Anneaux : rayon R = 1000 unités au centre de rotation ; seul le secteur visible est
# dessiné (symétrie d'ordre 24 : une rotation de 15 degrés redonne le même dessin).
VB_ANNEAUX = (80, -680, 620, 1360)
BANDES = [(0.42, 0.52, 2), (0.55, 0.57, 2), (0.61, 0.82, 1), (0.86, 0.9, 1)]
DENSITE = 3600


def anneaux(couleur):
    rnd = alea(17)
    couches = {1: {}, 2: {}}
    x0, y0, w, h = VB_ANNEAUX
    for a, b, tours in BANDES:
        n = round(DENSITE * (b - a))
        for _ in range(n):
            r = (a + (b - a) * rnd()) * 0.8 * 1000
            th = rnd() * 2 * math.pi / 24
            s = (0.45 + 1.6 * rnd() ** 2.8) * 0.8
            o = 0.15 + 0.6 * rnd()
            am = rnd() < 0.025
            cle = "ambre" if am else f"{min(3, int((o - 0.15) / 0.15)) * 0.15 + 0.25:.2f}"
            for j in range(24):
                an = th + j * 2 * math.pi / 24
                deg = math.degrees(math.atan2(math.sin(an), math.cos(an)))
                if not -72 <= deg <= 78:
                    continue
                x, y = r * math.cos(an), r * math.sin(an)
                if not (x0 <= x <= x0 + w and y0 <= y <= y0 + h):
                    continue
                couches[tours].setdefault(cle, []).append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{s:.2f}"/>')
    sorties, total = {}, 0
    for tours, groupes in couches.items():
        corps = ""
        for cle, cercles in sorted(groupes.items()):
            total += len(cercles)
            if cle == "ambre":
                corps += '<g fill="#E3A23B">' + "".join(cercles) + "</g>"
            else:
                corps += f'<g fill="{couleur}" fill-opacity="{cle}">' + "".join(cercles) + "</g>"
        sorties[tours] = svg(w, h, corps, f"{x0} {y0} {w} {h}")
    return sorties, total


def hachage(i, j):
    return (((i * 73856093) ^ (j * 19349663)) & 0xFFFFFFFF) % 4


def ecrire(nom, texte):
    (SORTIE / nom).write_text(texte, encoding="utf-8")


def slint(cases):
    def tableau(liste):
        return "[" + ", ".join(f"{{ x: {f(x)}px, y: {f(y)}px, k: {k}, r: {r:.3f} }}" for x, y, k, r in liste) + "]"

    tournant = []
    for j in range(4):
        for i in range(3):
            tournant.append(f"{{ i: {i}, j: {j}, h: {hachage(i, j)}, ambre: {'true' if (i, j) == (1, 2) else 'false'} }}")
    img = lambda n: f'@image-url("icones/motifs/{n}.svg")'
    piece = lambda var: (f"{var} == 0 ? {img('motif-piece-carre')} : {var} == 1 ? {img('motif-piece-quart')} : "
                         f"{var} == 2 ? {img('motif-piece-tri')} : {img('motif-piece-disque')}")
    fond = lambda var: (f"{var} == 0 ? {img('a2-fond-carre')} : {var} == 1 ? {img('a2-fond-quart')} : "
                        f"{var} == 2 ? {img('a2-fond-tri')} : {img('a2-fond-disque')}")
    reflet = " : ".join(f"k == {k} ? {img(f'a2-reflet-quart-{k}')}" for k in range(6)) + f" : {img('a2-reflet-quart-6')}"
    return f"""// Motifs animés adoptés le 2026-11-18 (docs/marque/README.md, section Identité animée).
// Fichier généré par docs/marque/propositions-2026-11-18-ecrans-animes/generer_motifs_slint.py :
// le modifier là, puis relancer le script.
//
// - MotifA2 : Trame de pièces. Une pièce du symbole se compose en petites pièces ambre
//   sur le motif Quatre pièces ton sur ton (entrée jouée une fois), reflet lent en option.
// - MotifAnneaux : anneaux de points qui tournent lentement, pièce ambre au centre.
// - MotifTournant : quarts de disque qui tournent d'un quart de tour à chaque étape.
// Sans animation (réglage « Animer les transitions... » coupé), les motifs restent fixes.
import {{ Affichage, Theme }} from "composants.slint";

// Couleurs des motifs : ton sur ton sur le fond du panneau (Theme.nuit), jamais de bleu
// en mode sombre ; seules les pièces vedettes sont en Ambre.
export global CouleursMotif {{
    out property <color> motif: !Theme.sombre ? #232c6b : Theme.palette == 0 ? #212327 : Theme.palette == 1 ? #1c232b : #22201e;
    out property <color> reflet: !Theme.sombre ? #2a3478 : Theme.palette == 0 ? #2a2d32 : Theme.palette == 1 ? #253039 : #2c2a27;
    out property <color> lignes: Theme.sombre ? Theme.texte : #f4f5f9;
}}

export struct CaseMotif {{ x: length, y: length, k: int, r: float }}
struct CaseTournant {{ i: int, j: int, h: int, ambre: bool }}

global DonneesMotif {{
    out property <[CaseMotif]> carre: {tableau(cases["carre"])};
    out property <[CaseMotif]> quart: {tableau(cases["quart"])};
    out property <[CaseMotif]> tri: {tableau(cases["tri"])};
    out property <[CaseMotif]> disque: {tableau(cases["disque"])};
    out property <[CaseTournant]> tournant: [{", ".join(tournant)}];
}}

// Entrée jouée une fois : t en secondes depuis l'apparition, figé à la fin (la fenêtre
// ne se redessine plus) ou d'emblée quand les animations sont coupées.
export component Entree inherits Rectangle {{
    width: 0px;
    height: 0px;
    in property <duration> longueur: 1900ms;
    out property <float> t: (self.fini || !Affichage.animations) ? 9 : (animation-tick() - self.depart) / 1s;
    property <duration> depart;
    property <bool> fini: false;
    init => {{
        self.depart = animation-tick();
    }}
    Timer {{
        interval: root.longueur;
        running: !root.fini;
        triggered => {{
            root.fini = true;
        }}
    }}
}}

// Trame de pièces sur un panneau (référence 640 x 760 posée en « cover »).
// piece : 0 carré, 1 quart de disque, 2 triangle, 3 disque.
export component MotifA2 inherits Rectangle {{
    in property <int> piece: 0;
    // Reflet très lent qui passe sur le motif (connexion ; quart de disque seulement).
    in property <bool> reflet: false;
    property <float> s: max(self.width / {W}px, self.height / {H}px);
    property <length> ox: (self.width - {W}px * self.s) / 2;
    property <length> oy: (self.height - {H}px * self.s) / 2;
    property <float> g: clamp((entree.t - 0.2) / 1.4, 0, 1) * 1.25;
    property <float> tr: Affichage.animations ? animation-tick() / 1s : 0;
    clip: true;
    // Taille imposée par l'écran : le motif ne réclame rien à la mise en page (sinon boucle).
    min-width: 0px;
    min-height: 0px;
    preferred-width: 0px;
    preferred-height: 0px;

    pure function rebond(x: float) -> float {{
        return 1 + 2.70158 * (x - 1) * (x - 1) * (x - 1) + 1.70158 * (x - 1) * (x - 1);
    }}

    entree := Entree {{ }}

    // Taille donnée par le motif, jamais en pourcentage : une image en 100 % remonte sa
    // hauteur préférée au panneau, qui dépend de sa largeur (boucle).
    Image {{
        x: 0;
        y: 0;
        width: root.width;
        height: root.height;
        image-fit: cover;
        colorize: CouleursMotif.motif;
        source: {fond("root.piece")};
    }}

    if root.reflet && root.piece == 1: Rectangle {{
        for k in 7: Image {{
            width: root.width;
            height: root.height;
            image-fit: cover;
            colorize: CouleursMotif.reflet;
            opacity: clamp((0.5 + 0.5 * sin((2 * k + 0.5) * 0.45rad - root.tr * 45deg) - 0.75) / 0.25, 0, 1);
            source: {reflet};
        }}
    }}

    for c in root.piece == 0 ? DonneesMotif.carre : root.piece == 1 ? DonneesMotif.quart : root.piece == 2 ? DonneesMotif.tri : DonneesMotif.disque: Image {{
        property <float> e: root.rebond(clamp((root.g - c.r) / 0.25, 0, 1));
        x: root.ox + c.x * root.s - self.width / 2;
        y: root.oy + c.y * root.s - self.height / 2;
        width: {f(TAILLE * 108 / 104)}px * root.s;
        height: self.width;
        visible: self.e > 0.01;
        colorize: Theme.ambre;
        source: {piece("c.k")};
        transform-scale-x: self.e * 100%;
        transform-scale-y: self.e * 100%;
        transform-origin: {{ x: self.width / 2, y: self.height / 2 }};
    }}
}}

// Anneaux de points autour d'un centre hors du panneau, à gauche ; ils tournent très
// lentement (un tour en 288 s, la couche intérieure deux fois plus vite). Zone nette
// en haut (logo) et en bas (« Gestion commerciale »).
export component MotifAnneaux inherits Rectangle {{
    in property <int> piece: 3;
    property <length> rayon: 1.15 * max(self.width, 1.4 * self.height);
    property <length> cx: -0.32 * self.width;
    property <length> cy: 0.36 * self.height;
    property <float> tours: Affichage.animations ? animation-tick() / 288s : 0;
    property <length> cote: 0.6 * min(self.width, 0.8 * self.height) * 0.62;
    clip: true;
    // Taille imposée par l'écran : le motif ne réclame rien à la mise en page (sinon boucle).
    min-width: 0px;
    min-height: 0px;
    preferred-width: 0px;
    preferred-height: 0px;

    pure function rebond(x: float) -> float {{
        return 1 + 2.70158 * (x - 1) * (x - 1) * (x - 1) + 1.70158 * (x - 1) * (x - 1);
    }}

    entree := Entree {{ longueur: 1300ms; }}

    Rectangle {{
        y: 100px;
        width: root.width;
        height: max(0px, root.height - 250px);
        clip: true;

        Image {{
            x: root.cx + {VB_ANNEAUX[0] / 1000} * root.rayon;
            y: root.cy + {VB_ANNEAUX[1] / 1000} * root.rayon - 100px;
            width: {VB_ANNEAUX[2] / 1000} * root.rayon;
            height: {VB_ANNEAUX[3] / 1000} * root.rayon;
            source: Theme.sombre ? {img('anneaux-1-sombre')} : {img('anneaux-1-clair')};
            transform-rotation: mod(root.tours * 24, 1) * 15deg;
            transform-origin: {{ x: {-VB_ANNEAUX[0] / 1000} * root.rayon, y: {-VB_ANNEAUX[1] / 1000} * root.rayon }};
        }}

        Image {{
            x: root.cx + {VB_ANNEAUX[0] / 1000} * root.rayon;
            y: root.cy + {VB_ANNEAUX[1] / 1000} * root.rayon - 100px;
            width: {VB_ANNEAUX[2] / 1000} * root.rayon;
            height: {VB_ANNEAUX[3] / 1000} * root.rayon;
            source: Theme.sombre ? {img('anneaux-2-sombre')} : {img('anneaux-2-clair')};
            transform-rotation: mod(root.tours * 48, 1) * 15deg;
            transform-origin: {{ x: {-VB_ANNEAUX[0] / 1000} * root.rayon, y: {-VB_ANNEAUX[1] / 1000} * root.rayon }};
        }}
    }}

    Image {{
        property <float> e: root.rebond(clamp((entree.t - 0.3) / 0.8, 0, 1));
        x: root.width / 2 - self.width / 2;
        y: 0.42 * root.height - self.height / 2;
        width: root.cote * 108 / 104;
        height: self.width;
        visible: self.e > 0.01;
        colorize: Theme.ambre;
        source: {piece("root.piece")};
        transform-scale-x: self.e * 100%;
        transform-scale-y: self.e * 100%;
        transform-origin: {{ x: self.width / 2, y: self.height / 2 }};
    }}
}}

// Quarts de disque au bas d'une colonne étroite (3 cases de large, 4 rangées) : chaque
// case avance d'un quart de tour quand etape change, en vague (décalage par diagonale).
export component MotifTournant inherits Rectangle {{
    in property <int> etape: 0;
    property <length> cote: self.width / 3;
    clip: true;
    // Taille imposée par l'écran : le motif ne réclame rien à la mise en page (sinon boucle).
    min-width: 0px;
    min-height: 0px;
    preferred-width: 0px;
    preferred-height: 0px;

    for c in DonneesMotif.tournant: Image {{
        x: c.i * root.cote;
        y: root.height - (4 - c.j) * root.cote;
        width: root.cote;
        height: root.cote;
        colorize: c.ambre ? Theme.ambre : CouleursMotif.motif;
        source: {img('tournant-quart')};
        transform-rotation: (c.h + root.etape) * 90deg;
        transform-origin: {{ x: self.width / 2, y: self.height / 2 }};
        animate transform-rotation {{
            duration: Affichage.animations ? 700ms : 0ms;
            delay: Affichage.animations ? (c.i + c.j) * 60ms : 0ms;
            easing: ease-in-out;
        }}
    }}
}}
"""


if __name__ == "__main__":
    SORTIE.mkdir(parents=True, exist_ok=True)
    for k in ORDRE:
        px, py = CENTRE[k]
        ecrire(f"motif-piece-{k}.svg", svg(108, 108, f'<path fill="#fff" transform="translate({52 - px} {52 - py})" d="{PIECES[k]}"/>', "-2 -2 108 108"))
    ecrire("tournant-quart.svg", svg(104, 104, f'<path fill="#fff" d="{QUART_TOURNANT}"/>'))
    cases = {}
    for k in ORDRE:
        ambre, fond, reflets = a2(k)
        cases[k] = ambre
        ecrire(f"a2-fond-{k}.svg", fond)
        if k == "quart":
            for n, r in enumerate(reflets):
                ecrire(f"a2-reflet-quart-{n}.svg", r)
        print(k, len(ambre), "cases ambre")
    for mode, couleur in (("clair", "#F4F5F9"), ("sombre", "#EEEBE7")):
        couches, total = anneaux(couleur)
        for tours, texte in couches.items():
            ecrire(f"anneaux-{tours}-{mode}.svg", texte)
        print("anneaux", mode, total, "points")
    (UI / "motifs-ecrans.slint").write_text(slint(cases), encoding="utf-8")
    for p in sorted(SORTIE.iterdir()):
        print(p.name, p.stat().st_size)
