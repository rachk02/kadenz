"""Génère les modèles Excel d'import livrés dans le dossier modeles.

Un fichier .xlsx est une archive zip de fichiers XML : on l'écrit à la main,
sans dépendance. Les en-têtes sont les mêmes que ceux des modèles CSV voisins
(installateur/contenu/modeles). Relancer ce script après tout changement des
colonnes attendues par l'import, puis commiter les .xlsx produits.

Usage : python generer_modeles_xlsx.py
"""
import io
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

SORTIE = Path(__file__).parent / "contenu" / "modeles"

MODELES = {
    "modele-tiers.xlsx": ("Tiers", ["Nom", "Adresse", "Téléphone", "Localisation", "IFU", "RCCM", "Régime d'imposition", "Type"]),
    "modele-produits.xlsx": ("Produits", ["Désignation", "Référence", "Prix", "Sens"]),
}


def lettre(i: int) -> str:
    return chr(ord("A") + i)


def classeur(feuille: str, colonnes: list[str]) -> bytes:
    cellules = "".join(
        f'<c r="{lettre(i)}1" t="inlineStr" s="1"><is><t>{escape(nom)}</t></is></c>'
        for i, nom in enumerate(colonnes)
    )
    largeurs = "".join(
        f'<col min="{i + 1}" max="{i + 1}" width="{max(14, len(nom) + 4)}" customWidth="1"/>'
        for i, nom in enumerate(colonnes)
    )
    parties = {
        "[Content_Types].xml": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
            '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
            '<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>'
            "</Types>"
        ),
        "_rels/.rels": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
            "</Relationships>"
        ),
        "xl/workbook.xml": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets><sheet name="{escape(feuille)}" sheetId="1" r:id="rId1"/></sheets></workbook>'
        ),
        "xl/_rels/workbook.xml.rels": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>'
            '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
            "</Relationships>"
        ),
        "xl/styles.xml": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            '<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font>'
            '<font><b/><sz val="11"/><name val="Calibri"/></font></fonts>'
            '<fills count="2"><fill><patternFill patternType="none"/></fill>'
            '<fill><patternFill patternType="gray125"/></fill></fills>'
            '<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>'
            '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
            '<cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'
            '<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs>'
            "</styleSheet>"
        ),
        "xl/worksheets/sheet1.xml": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            f"<cols>{largeurs}</cols>"
            f'<sheetData><row r="1">{cellules}</row></sheetData></worksheet>'
        ),
    }
    tampon = io.BytesIO()
    with zipfile.ZipFile(tampon, "w", zipfile.ZIP_DEFLATED) as z:
        for nom, contenu in parties.items():
            z.writestr(nom, contenu.encode("utf-8"))
    return tampon.getvalue()


if __name__ == "__main__":
    for fichier, (feuille, colonnes) in MODELES.items():
        (SORTIE / fichier).write_bytes(classeur(feuille, colonnes))
        print("écrit", SORTIE / fichier)
