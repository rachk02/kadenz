"""Génère le thème de l'installateur KadenzSetup (WixStdBA, WiX 5).

Sorties, dans ce dossier, versionnées avec le dépôt (la construction de
l'installateur n'a pas besoin de Python) :
- kadenz.thm : pages de l'assistant ; chaque texte est décliné dans les sept
  langues de Kadenz, affiché selon la variable KadenzLangue choisie sur la
  première page ;
- kadenz.wxl : chaînes de ces textes, et chaînes que WixStdBA utilise
  lui-même (confirmation d'annulation, fichiers en cours d'utilisation) ;
- logo.png et logoside.png : identité de Kadenz, tirée de crates/app/kadenz.ico,
  couleurs et police Manrope de l'application (crates/app/ui).

Usage : python installateur/theme/generer.py (Pillow requis).
"""

from pathlib import Path
from xml.sax.saxutils import escape, quoteattr

from PIL import Image, ImageDraw, ImageFont

ICI = Path(__file__).resolve().parent
RACINE = ICI.parent.parent

# Langues de l'application, dans l'ordre de son sélecteur, avec leur nom
# dans leur propre langue.
LANGUES = [
    ("fr", "Français"),
    ("en", "English"),
    ("ar", "العربية"),
    ("ja", "日本語"),
    ("zh", "中文"),
    ("tr", "Türkçe"),
    ("es", "Español"),
]

# Couleurs de l'application (crates/app/ui/composants.slint, global Theme).
NUIT = "#1a2259"
ACCENT = "#2d3a8c"
TEXTE_SECONDAIRE = "#52606d"

TEXTES = {
    "Bienvenue": {
        "fr": "Bienvenue dans l'installation de Kadenz",
        "en": "Welcome to Kadenz Setup",
        "ar": "مرحبًا بك في تثبيت Kadenz",
        "ja": "Kadenz のセットアップへようこそ",
        "zh": "欢迎使用 Kadenz 安装程序",
        "tr": "Kadenz kurulumuna hoş geldiniz",
        "es": "Bienvenido a la instalación de Kadenz",
    },
    "Intro": {
        "fr": "Kadenz, logiciel de gestion commerciale, va être installé sur cet ordinateur.",
        "en": "Kadenz, business management software, will be installed on this computer.",
        "ar": "سيتم تثبيت Kadenz، برنامج الإدارة التجارية، على هذا الحاسوب.",
        "ja": "販売管理ソフト Kadenz をこのパソコンにインストールします。",
        "zh": "将在此计算机上安装商务管理软件 Kadenz。",
        "tr": "Ticari yönetim yazılımı Kadenz bu bilgisayara kurulacak.",
        "es": "Kadenz, software de gestión comercial, se instalará en este equipo.",
    },
    "Dossier": {
        "fr": "Dossier d'installation :",
        "en": "Installation folder:",
        "ar": "مجلد التثبيت:",
        "ja": "インストール先フォルダー:",
        "zh": "安装文件夹：",
        "tr": "Kurulum klasörü:",
        "es": "Carpeta de instalación:",
    },
    "Parcourir": {
        "fr": "Parcourir...",
        "en": "Browse...",
        "ar": "استعراض...",
        "ja": "参照...",
        "zh": "浏览...",
        "tr": "Gözat...",
        "es": "Examinar...",
    },
    "Bureau": {
        "fr": "Créer un raccourci sur le bureau",
        "en": "Create a desktop shortcut",
        "ar": "إنشاء اختصار على سطح المكتب",
        "ja": "デスクトップにショートカットを作成する",
        "zh": "在桌面创建快捷方式",
        "tr": "Masaüstüne kısayol oluştur",
        "es": "Crear un acceso directo en el escritorio",
    },
    "Menu": {
        "fr": "Ajouter Kadenz au menu Démarrer",
        "en": "Add Kadenz to the Start menu",
        "ar": "إضافة Kadenz إلى قائمة ابدأ",
        "ja": "スタート メニューに Kadenz を追加する",
        "zh": "将 Kadenz 添加到“开始”菜单",
        "tr": "Kadenz'i Başlat menüsüne ekle",
        "es": "Añadir Kadenz al menú Inicio",
    },
    "Donnees": {
        "fr": "Les données de l'entreprise sont enregistrées dans le profil de chaque compte Windows ; une désinstallation ne les supprime pas.",
        "en": "Company data is stored in each Windows account's profile; uninstalling does not delete it.",
        "ar": "تُحفظ بيانات الشركة في ملف تعريف كل حساب Windows، ولا تؤدي إزالة التثبيت إلى حذفها.",
        "ja": "会社のデータは各 Windows アカウントのプロファイルに保存され、アンインストールしても削除されません。",
        "zh": "公司数据保存在每个 Windows 帐户的个人资料中，卸载不会删除这些数据。",
        "tr": "Şirket verileri her Windows hesabının profilinde saklanır; kaldırma işlemi bunları silmez.",
        "es": "Los datos de la empresa se guardan en el perfil de cada cuenta de Windows; la desinstalación no los elimina.",
    },
    "Retour": {"fr": "Retour", "en": "Back", "ar": "السابق", "ja": "戻る", "zh": "上一步", "tr": "Önceki", "es": "Anterior"},
    "Suivant": {"fr": "Suivant", "en": "Next", "ar": "التالي", "ja": "次へ", "zh": "下一步", "tr": "Sonraki", "es": "Siguiente"},
    "Installer": {"fr": "Installer", "en": "Install", "ar": "تثبيت", "ja": "インストール", "zh": "安装", "tr": "Yükle", "es": "Instalar"},
    "Annuler": {"fr": "Annuler", "en": "Cancel", "ar": "إلغاء", "ja": "キャンセル", "zh": "取消", "tr": "İptal", "es": "Cancelar"},
    "Fermer": {"fr": "Fermer", "en": "Close", "ar": "إغلاق", "ja": "閉じる", "zh": "关闭", "tr": "Kapat", "es": "Cerrar"},
    "Lancer": {
        "fr": "Lancer Kadenz",
        "en": "Launch Kadenz",
        "ar": "تشغيل Kadenz",
        "ja": "Kadenz を起動",
        "zh": "启动 Kadenz",
        "tr": "Kadenz'i başlat",
        "es": "Iniciar Kadenz",
    },
    "Redemarrer": {"fr": "Redémarrer", "en": "Restart", "ar": "إعادة التشغيل", "ja": "再起動", "zh": "重新启动", "tr": "Yeniden başlat", "es": "Reiniciar"},
    "Reparer": {"fr": "Réparer", "en": "Repair", "ar": "إصلاح", "ja": "修復", "zh": "修复", "tr": "Onar", "es": "Reparar"},
    "Desinstaller": {"fr": "Désinstaller", "en": "Uninstall", "ar": "إزالة التثبيت", "ja": "アンインストール", "zh": "卸载", "tr": "Kaldır", "es": "Desinstalar"},
    "EnCours": {
        "fr": "Installation en cours",
        "en": "Installing",
        "ar": "جارٍ التثبيت",
        "ja": "インストールしています",
        "zh": "正在安装",
        "tr": "Kuruluyor",
        "es": "Instalando",
    },
    "DesinstallationEnCours": {
        "fr": "Désinstallation en cours",
        "en": "Uninstalling",
        "ar": "جارٍ إزالة التثبيت",
        "ja": "アンインストールしています",
        "zh": "正在卸载",
        "tr": "Kaldırılıyor",
        "es": "Desinstalando",
    },
    "ReparationEnCours": {
        "fr": "Réparation en cours",
        "en": "Repairing",
        "ar": "جارٍ الإصلاح",
        "ja": "修復しています",
        "zh": "正在修复",
        "tr": "Onarılıyor",
        "es": "Reparando",
    },
    "Installe": {
        "fr": "Kadenz est installé",
        "en": "Kadenz is installed",
        "ar": "تم تثبيت Kadenz",
        "ja": "Kadenz のインストールが完了しました",
        "zh": "Kadenz 已安装",
        "tr": "Kadenz kuruldu",
        "es": "Kadenz se ha instalado",
    },
    "Desinstalle": {
        "fr": "Kadenz est désinstallé",
        "en": "Kadenz has been uninstalled",
        "ar": "تمت إزالة تثبيت Kadenz",
        "ja": "Kadenz をアンインストールしました",
        "zh": "Kadenz 已卸载",
        "tr": "Kadenz kaldırıldı",
        "es": "Kadenz se ha desinstalado",
    },
    "Repare": {
        "fr": "Kadenz est réparé",
        "en": "Kadenz has been repaired",
        "ar": "تم إصلاح Kadenz",
        "ja": "Kadenz を修復しました",
        "zh": "Kadenz 已修复",
        "tr": "Kadenz onarıldı",
        "es": "Kadenz se ha reparado",
    },
    "Echec": {
        "fr": "L'installation a échoué",
        "en": "Setup failed",
        "ar": "فشل التثبيت",
        "ja": "インストールに失敗しました",
        "zh": "安装失败",
        "tr": "Kurulum başarısız oldu",
        "es": "La instalación ha fallado",
    },
    "EchecDesinstallation": {
        "fr": "La désinstallation a échoué",
        "en": "Uninstall failed",
        "ar": "فشلت إزالة التثبيت",
        "ja": "アンインストールに失敗しました",
        "zh": "卸载失败",
        "tr": "Kaldırma başarısız oldu",
        "es": "La desinstalación ha fallado",
    },
    "Journal": {
        "fr": 'Pour en savoir plus, ouvrez le <a href="#">journal d\'installation</a>.',
        "en": 'For more information, open the <a href="#">setup log</a>.',
        "ar": 'لمزيد من المعلومات، افتح <a href="#">سجل التثبيت</a>.',
        "ja": '詳しくは<a href="#">セットアップ ログ</a>を開いてください。',
        "zh": '有关详细信息，请打开<a href="#">安装日志</a>。',
        "tr": 'Daha fazla bilgi için <a href="#">kurulum günlüğünü</a> açın.',
        "es": 'Para más información, abra el <a href="#">registro de instalación</a>.',
    },
    "RedemarrageRequis": {
        "fr": "Redémarrez l'ordinateur pour terminer.",
        "en": "Restart the computer to finish.",
        "ar": "أعد تشغيل الحاسوب لإكمال العملية.",
        "ja": "完了するにはパソコンを再起動してください。",
        "zh": "请重新启动计算机以完成操作。",
        "tr": "Tamamlamak için bilgisayarı yeniden başlatın.",
        "es": "Reinicie el equipo para terminar.",
    },
    "Maintenance": {
        "fr": "Que voulez-vous faire ?",
        "en": "What would you like to do?",
        "ar": "ماذا تريد أن تفعل؟",
        "ja": "実行する操作を選んでください。",
        "zh": "您想执行什么操作？",
        "tr": "Ne yapmak istiyorsunuz?",
        "es": "¿Qué desea hacer?",
    },
    "Aide": {
        "fr": "Options de la ligne de commande : /install, /repair, /uninstall, /quiet, /passive, /log fichier.",
        "en": "Command line options: /install, /repair, /uninstall, /quiet, /passive, /log file.",
        "ar": "خيارات سطر الأوامر: /install و/repair و/uninstall و/quiet و/passive و/log ملف.",
        "ja": "コマンド ライン オプション: /install、/repair、/uninstall、/quiet、/passive、/log ファイル。",
        "zh": "命令行选项：/install、/repair、/uninstall、/quiet、/passive、/log 文件。",
        "tr": "Komut satırı seçenekleri: /install, /repair, /uninstall, /quiet, /passive, /log dosya.",
        "es": "Opciones de la línea de comandos: /install, /repair, /uninstall, /quiet, /passive, /log archivo.",
    },
}

# Chaînes lues par WixStdBA lui-même, hors thème : une seule valeur possible,
# en français et en anglais.
CHAINES_WIXSTDBA = {
    "Caption": "Kadenz",
    "ConfirmCancelMessage": "Voulez-vous vraiment annuler ? / Are you sure you want to cancel?",
    "ExecuteUpgradeRelatedBundleMessage": "Version précédente / Previous version",
    "OverallProgressPackageText": "[ProgressPackageName]",
    "FilesInUseTitle": "Fichiers en cours d'utilisation / Files in use",
    "FilesInUseLabel": "Ces applications utilisent des fichiers à mettre à jour : / These applications are using files that need to be updated:",
    "FilesInUseNetfxCloseRadioButton": "Fermer les applications / Close the applications",
    "FilesInUseCloseRadioButton": "Fermer les applications et les relancer / Close and restart the applications",
    "FilesInUseDontCloseRadioButton": "Ne pas fermer (redémarrage nécessaire) / Do not close (restart required)",
    "FilesInUseRetryButton": "Réessayer / Retry",
    "FilesInUseIgnoreButton": "Ignorer / Ignore",
    "FilesInUseExitButton": "Quitter / Exit",
}

PAGE_LANGUE_TITRE = "Langue d'installation · Installation language"


def cle(nom, langue):
    return f"{langue}_{nom}"


def textes(nom):
    """Éléments Text d'un contrôle : le français par défaut, puis chaque langue
    sous condition (la dernière condition vraie l'emporte)."""
    lignes = [f"<Text>#(loc.{cle(nom, 'fr')})</Text>"]
    for code, _ in LANGUES:
        condition = quoteattr(f'KadenzLangue = "{code}"')
        lignes.append(f"<Text Condition={condition}>#(loc.{cle(nom, code)})</Text>")
    return "".join(lignes)


def bgr(couleur):
    r, g, b = couleur[1:3], couleur[3:5], couleur[5:7]
    return f"{b}{g}{r}"


COTE = '<ImageControl X="0" Y="0" Width="180" Height="0" ImageFile="logoside.png" />'
X = 200  # début de la zone de contenu, à droite de l'image


def titre(nom, y=24):
    return f'<Label X="{X}" Y="{y}" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes(nom)}</Label>'


def bouton(nom_controle, texte_cle, x, largeur=96, action="", condition=""):
    attributs = f'Name="{nom_controle}" X="{x}" Y="-16" Width="{largeur}" Height="28" TabStop="yes" FontId="0"'
    if condition:
        attributs += f" {condition}"
    return f"<Button {attributs}>{textes(texte_cle)}{action}</Button>"


def theme():
    radios = []
    for i, (code, nom) in enumerate(LANGUES):
        radios.append(
            f'<RadioButton Value="{code}" X="{X + 8}" Y="{120 + i * 34}" Width="-20" Height="26" '
            f'TabStop="{"yes" if i == 0 else "no"}" FontId="3"><Text>{escape(nom)}</Text></RadioButton>'
        )
    pages = [
        f'<Page Name="Loading">{COTE}</Page>',
        f'<Page Name="Help">{COTE}{titre("Bienvenue")}'
        f'<Label X="{X}" Y="110" Width="-20" Height="-60" FontId="3" DisablePrefix="yes">{textes("Aide")}</Label>'
        + bouton("HelpCloseButton", "Fermer", -20, action="<CloseWindowAction />")
        + "</Page>",
        # Première page : choix de la langue, noms des langues dans leur langue.
        f'<Page Name="Install">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="40" FontId="1" DisablePrefix="yes">Kadenz</Label>'
        f'<Label X="{X}" Y="76" Width="-20" Height="30" FontId="2" DisablePrefix="yes">{escape(PAGE_LANGUE_TITRE)}</Label>'
        f'<RadioButtons Name="KadenzLangue">{"".join(radios)}</RadioButtons>'
        + bouton("SuivantButton", "Suivant", -124, action='<ChangePageAction Page="Options" />')
        + bouton("InstallCancelButton", "Annuler", -20, action="<CloseWindowAction />")
        + "</Page>",
        # Deuxième page : dossier, raccourcis, puis installation.
        f'<Page Name="Options">{COTE}{titre("Bienvenue")}'
        f'<Label X="{X}" Y="96" Width="-20" Height="40" FontId="3" DisablePrefix="yes">{textes("Intro")}</Label>'
        f'<Label X="{X}" Y="150" Width="-20" Height="20" FontId="3" DisablePrefix="yes">{textes("Dossier")}</Label>'
        f'<Editbox Name="InstallFolder" X="{X}" Y="174" Width="-124" Height="24" TabStop="yes" FontId="3" FileSystemAutoComplete="yes" />'
        f'<Button Name="BrowseButton" X="-20" Y="173" Width="96" Height="26" TabStop="yes" FontId="3">'
        f'{textes("Parcourir")}<BrowseDirectoryAction VariableName="InstallFolder" /></Button>'
        f'<Checkbox Name="RaccourciBureau" X="{X}" Y="220" Width="-20" Height="24" TabStop="yes" FontId="3">{textes("Bureau")}</Checkbox>'
        f'<Checkbox Name="RaccourciMenu" X="{X}" Y="250" Width="-20" Height="24" TabStop="yes" FontId="3">{textes("Menu")}</Checkbox>'
        f'<Label X="{X}" Y="292" Width="-20" Height="60" FontId="4" DisablePrefix="yes">{textes("Donnees")}</Label>'
        + bouton("RetourButton", "Retour", -228, action='<ChangePageAction Page="Install" />')
        + bouton("InstallButton", "Installer", -124)
        + bouton("OptionsCancelButton", "Annuler", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Progress">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("EnCours")}'
        + "".join(
            f'<Text Condition={quoteattr(f"(WixBundleAction = 3 OR WixBundleAction = 4) AND KadenzLangue = {chr(34)}{c}{chr(34)}")}>#(loc.{cle("DesinstallationEnCours", c)})</Text>'
            for c, _ in LANGUES
        )
        + "".join(
            f'<Text Condition={quoteattr(f"WixBundleAction = 8 AND KadenzLangue = {chr(34)}{c}{chr(34)}")}>#(loc.{cle("ReparationEnCours", c)})</Text>'
            for c, _ in LANGUES
        )
        + "</Label>"
        f'<Label Name="OverallProgressPackageText" X="{X}" Y="110" Width="-20" Height="20" FontId="3" DisablePrefix="yes">#(loc.OverallProgressPackageText)</Label>'
        f'<Progressbar Name="OverallCalculatedProgressbar" X="{X}" Y="136" Width="-20" Height="18" />'
        + bouton("ProgressCancelButton", "Annuler", -20)
        + "</Page>",
        f'<Page Name="Modify">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="40" FontId="1" DisablePrefix="yes">Kadenz</Label>'
        f'<Label X="{X}" Y="76" Width="-20" Height="30" FontId="2" DisablePrefix="yes">{textes("Maintenance")}</Label>'
        f'<Label X="{X}" Y="120" Width="-20" Height="60" FontId="4" DisablePrefix="yes">{textes("Donnees")}</Label>'
        + bouton("RepairButton", "Reparer", -228, condition='HideWhenDisabled="yes"')
        + bouton("UninstallButton", "Desinstaller", -124)
        + bouton("ModifyCancelButton", "Annuler", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Success">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("Installe")}'
        + "".join(
            f'<Text Condition={quoteattr(f"(WixBundleAction = 3 OR WixBundleAction = 4) AND KadenzLangue = {chr(34)}{c}{chr(34)}")}>#(loc.{cle("Desinstalle", c)})</Text>'
            for c, _ in LANGUES
        )
        + "".join(
            f'<Text Condition={quoteattr(f"WixBundleAction = 8 AND KadenzLangue = {chr(34)}{c}{chr(34)}")}>#(loc.{cle("Repare", c)})</Text>'
            for c, _ in LANGUES
        )
        + "</Label>"
        f'<Label X="{X}" Y="110" Width="-20" Height="60" FontId="4" DisablePrefix="yes">{textes("Donnees")}</Label>'
        f'<Label X="{X}" Y="-60" Width="-20" Height="34" FontId="3" DisablePrefix="yes" VisibleCondition="WixStdBARestartRequired">{textes("RedemarrageRequis")}</Label>'
        + bouton("LaunchButton", "Lancer", -124, largeur=140, condition='HideWhenDisabled="yes"')
        + bouton("SuccessRestartButton", "Redemarrer", -124, largeur=140, condition='HideWhenDisabled="yes"')
        + bouton("SuccessCloseButton", "Fermer", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Failure">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("Echec")}'
        + "".join(
            f'<Text Condition={quoteattr(f"(WixBundleAction = 3 OR WixBundleAction = 4) AND KadenzLangue = {chr(34)}{c}{chr(34)}")}>#(loc.{cle("EchecDesinstallation", c)})</Text>'
            for c, _ in LANGUES
        )
        + "</Label>"
        f'<Hypertext Name="FailureLogFileLink" X="{X}" Y="110" Width="-20" Height="40" FontId="3" TabStop="yes" HideWhenDisabled="yes">{textes("Journal")}</Hypertext>'
        f'<Hypertext Name="FailureMessageText" X="{X}" Y="160" Width="-20" Height="120" FontId="3" TabStop="yes" HideWhenDisabled="yes" />'
        f'<Label X="{X}" Y="-60" Width="-20" Height="34" FontId="3" DisablePrefix="yes" VisibleCondition="WixStdBARestartRequired">{textes("RedemarrageRequis")}</Label>'
        + bouton("FailureRestartButton", "Redemarrer", -124, largeur=140, condition='HideWhenDisabled="yes"')
        + bouton("FailureCloseButton", "Fermer", -20, action="<CloseWindowAction />")
        + "</Page>",
    ]
    polices = (
        f'<Font Id="0" Height="-13" Weight="400" Foreground="windowtext" Background="ffffff">Segoe UI</Font>'
        f'<Font Id="1" Height="-26" Weight="600" Foreground="{bgr(NUIT)}" Background="ffffff">Segoe UI</Font>'
        f'<Font Id="2" Height="-17" Weight="400" Foreground="{bgr(ACCENT)}" Background="ffffff">Segoe UI</Font>'
        f'<Font Id="3" Height="-14" Weight="400" Foreground="windowtext" Background="ffffff">Segoe UI</Font>'
        f'<Font Id="4" Height="-12" Weight="400" Foreground="{bgr(TEXTE_SECONDAIRE)}" Background="ffffff">Segoe UI</Font>'
    )
    corps = "\n    ".join(pages)
    return (
        '<?xml version="1.0" encoding="utf-8"?>\n'
        "<!-- Généré par installateur/theme/generer.py : modifier ce script, pas ce fichier. -->\n"
        '<Theme xmlns="http://wixtoolset.org/schemas/v4/thmutil">\n  '
        + polices
        + '\n  <Window Width="620" Height="460" HexStyle="100a0000" FontId="0" IconFile="kadenz.ico" Caption="#(loc.Caption)">\n    '
        + corps
        + "\n  </Window>\n</Theme>\n"
    )


def localisation():
    lignes = [
        '<?xml version="1.0" encoding="utf-8"?>',
        "<!-- Généré par installateur/theme/generer.py : modifier ce script, pas ce fichier. -->",
        '<WixLocalization Culture="fr-FR" Language="1036" xmlns="http://wixtoolset.org/schemas/v4/wxl">',
    ]
    for nom, valeur in CHAINES_WIXSTDBA.items():
        lignes.append(f"  <String Id={quoteattr(nom)} Value={quoteattr(valeur)} />")
    for nom, traductions in TEXTES.items():
        for code, _ in LANGUES:
            lignes.append(f"  <String Id={quoteattr(cle(nom, code))} Value={quoteattr(traductions[code])} />")
    lignes.append("</WixLocalization>")
    return "\n".join(lignes) + "\n"


def images():
    icone = Image.open(RACINE / "crates" / "app" / "kadenz.ico")
    icone.size = (256, 256)
    icone.load()
    icone = icone.convert("RGBA")
    icone.resize((64, 64), Image.LANCZOS).save(ICI / "logo.png")

    # Bandeau latéral : fond « nuit » de l'application, icône, nom en Manrope.
    largeur, hauteur = 180, 460
    cote = Image.new("RGBA", (largeur, hauteur), NUIT)
    taille = 96
    cote.alpha_composite(icone.resize((taille, taille), Image.LANCZOS), ((largeur - taille) // 2, 60))
    dessin = ImageDraw.Draw(cote)
    police = ImageFont.truetype(str(RACINE / "crates" / "app" / "ui" / "polices" / "Manrope-ExtraBold.ttf"), 30)
    boite = dessin.textbbox((0, 0), "Kadenz", font=police)
    dessin.text(((largeur - (boite[2] - boite[0])) // 2 - boite[0], 176), "Kadenz", font=police, fill="white")
    cote.convert("RGB").save(ICI / "logoside.png")


def main():
    (ICI / "kadenz.thm").write_text(theme(), encoding="utf-8")
    (ICI / "kadenz.wxl").write_text(localisation(), encoding="utf-8")
    images()
    print("Thème généré dans", ICI)


if __name__ == "__main__":
    main()
