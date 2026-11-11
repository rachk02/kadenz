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
    "Precedent": {"fr": "Précédent", "en": "Back", "ar": "السابق", "ja": "戻る", "zh": "上一步", "tr": "Önceki", "es": "Anterior"},
    "Suivant": {"fr": "Suivant", "en": "Next", "ar": "التالي", "ja": "次へ", "zh": "下一步", "tr": "Sonraki", "es": "Siguiente"},
    "Installer": {"fr": "Installer", "en": "Install", "ar": "تثبيت", "ja": "インストール", "zh": "安装", "tr": "Yükle", "es": "Instalar"},
    "Annuler": {"fr": "Annuler", "en": "Cancel", "ar": "إلغاء", "ja": "キャンセル", "zh": "取消", "tr": "İptal", "es": "Cancelar"},
    "Fermer": {"fr": "Fermer", "en": "Close", "ar": "إغلاق", "ja": "閉じる", "zh": "关闭", "tr": "Kapat", "es": "Cerrar"},
    "Terminer": {"fr": "Terminer", "en": "Finish", "ar": "إنهاء", "ja": "完了", "zh": "完成", "tr": "Bitir", "es": "Finalizar"},
    # Sous-titre de Kadenz, repris des traductions de l'application
    # (crates/app/lang/*/LC_MESSAGES/kadenz.po).
    "Slogan": {
        "fr": "Gestion commerciale",
        "en": "Sales management",
        "ar": "الإدارة التجارية",
        "ja": "販売管理",
        "zh": "商务管理",
        "tr": "Ticari yönetim",
        "es": "Gestión comercial",
    },
    "InstalleDans": {
        "fr": "Kadenz est installé dans :",
        "en": "Kadenz has been installed in:",
        "ar": "تم تثبيت Kadenz في:",
        "ja": "Kadenz のインストール先:",
        "zh": "Kadenz 已安装到：",
        "tr": "Kadenz'in kurulduğu klasör:",
        "es": "Kadenz se ha instalado en:",
    },
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
        "fr": "Installation terminée",
        "en": "Installation complete",
        "ar": "اكتمل التثبيت",
        "ja": "インストールが完了しました",
        "zh": "安装完成",
        "tr": "Kurulum tamamlandı",
        "es": "Instalación completada",
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


def boutons_radio(condition=""):
    """Langues de Kadenz, nommées dans leur propre langue, liées à KadenzLangue."""
    radios = []
    for i, (code, nom) in enumerate(LANGUES):
        radios.append(
            f'<RadioButton Value="{code}" X="{X + 8}" Y="{116 + i * 32}" Width="-20" Height="26" '
            f'TabStop="{"yes" if i == 0 else "no"}" FontId="3"{condition}><Text>{escape(nom)}</Text></RadioButton>'
        )
    return f'<RadioButtons Name="KadenzLangue">{"".join(radios)}</RadioButtons>'


# WixStdBA ne suit pas les changements de page faits par le thème : la page
# masquée au début de l'installation est toujours « Install ». La langue, puis
# le dossier et l'installation, sont donc deux étapes de cette même page,
# selon KadenzEtape. Un bouton du thème ne sait pas changer une variable : ce
# sont Suivant et Précédent, via l'extension (installateur/extension), qui la
# passent à 2 ou à 1 avant que leur ChangePageAction réaffiche la page. Choisir
# une langue traduit aussitôt la page, sans changer d'étape.
ETAPE_LANGUE = ' VisibleCondition="KadenzEtape = 1"'
ETAPE_DOSSIER = ' VisibleCondition="KadenzEtape = 2"'
# WixBundleAction : 3 et 4 désinstallation, 8 réparation (BOOTSTRAPPER_ACTION).
PAS_DESINSTALLATION = 'NOT (WixBundleAction = 3 OR WixBundleAction = 4)'
DESINSTALLATION = "WixBundleAction = 3 OR WixBundleAction = 4"
REPARATION = "WixBundleAction = 8"


def selon_action(condition, nom):
    """Textes d'un titre pour une action (désinstallation, réparation), placés
    après ses textes d'installation : le français pour l'action seule, puis
    chaque langue (la dernière condition vraie l'emporte)."""
    lignes = [f"<Text Condition={quoteattr(condition)}>#(loc.{cle(nom, 'fr')})</Text>"]
    for code, _ in LANGUES:
        lignes.append(
            f"<Text Condition={quoteattr(f'({condition}) AND KadenzLangue = \"{code}\"')}>#(loc.{cle(nom, code)})</Text>"
        )
    return "".join(lignes)


def theme():
    L, E = ETAPE_LANGUE, ETAPE_DOSSIER
    pages = [
        f'<Page Name="Loading">{COTE}</Page>',
        f'<Page Name="Help">{COTE}{titre("Bienvenue")}'
        f'<Label X="{X}" Y="110" Width="-20" Height="-60" FontId="3" DisablePrefix="yes">{textes("Aide")}</Label>'
        + bouton("HelpCloseButton", "Fermer", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Install">{COTE}'
        # Étape 1 : choix de la langue ; Suivant seulement, actif une fois la
        # langue choisie.
        f'<Label X="{X}" Y="24" Width="-20" Height="40" FontId="1" DisablePrefix="yes"{L}>Kadenz</Label>'
        f'<Label X="{X}" Y="76" Width="-20" Height="30" FontId="2" DisablePrefix="yes"{L}>{escape(PAGE_LANGUE_TITRE)}</Label>'
        + boutons_radio(L)
        + bouton("SuivantButton", "Suivant", -20, action='<ChangePageAction Page="Install" />',
                 condition=f'{L.strip()} EnableCondition="KadenzLangue"')
        # Étape 2 : dossier, raccourcis, installation, dans la langue choisie.
        + f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes"{E}>{textes("Bienvenue")}</Label>'
        f'<Label X="{X}" Y="96" Width="-20" Height="44" FontId="3" DisablePrefix="yes"{E}>{textes("Intro")}</Label>'
        f'<Label X="{X}" Y="150" Width="-20" Height="20" FontId="3" DisablePrefix="yes"{E}>{textes("Dossier")}</Label>'
        f'<Editbox Name="InstallFolder" X="{X}" Y="174" Width="-124" Height="24" TabStop="yes" FontId="3" FileSystemAutoComplete="yes"{E} />'
        f'<Button Name="BrowseButton" X="-20" Y="173" Width="96" Height="26" TabStop="yes" FontId="3"{E}>'
        f'{textes("Parcourir")}<BrowseDirectoryAction VariableName="InstallFolder" /></Button>'
        f'<Checkbox Name="RaccourciBureau" X="{X}" Y="220" Width="-20" Height="24" TabStop="yes" FontId="3"{E}>{textes("Bureau")}</Checkbox>'
        f'<Checkbox Name="RaccourciMenu" X="{X}" Y="250" Width="-20" Height="24" TabStop="yes" FontId="3"{E}>{textes("Menu")}</Checkbox>'
        + bouton("PrecedentButton", "Precedent", -124, action='<ChangePageAction Page="Install" />', condition=E.strip())
        + bouton("InstallButton", "Installer", -20, condition=E.strip())
        + "</Page>",
        f'<Page Name="Progress">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("EnCours")}'
        + selon_action(DESINSTALLATION, "DesinstallationEnCours")
        + selon_action(REPARATION, "ReparationEnCours")
        + "</Label>"
        f'<Label Name="OverallProgressPackageText" X="{X}" Y="110" Width="-20" Height="20" FontId="3" DisablePrefix="yes">#(loc.OverallProgressPackageText)</Label>'
        f'<Progressbar Name="OverallCalculatedProgressbar" X="{X}" Y="136" Width="-20" Height="18" />'
        # Détail : action de Windows Installer en cours.
        f'<Label Name="ExecuteProgressActionDataText" X="{X}" Y="166" Width="-20" Height="60" FontId="4" DisablePrefix="yes" />'
        + bouton("ProgressCancelButton", "Annuler", -20)
        + "</Page>",
        f'<Page Name="Modify">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="40" FontId="1" DisablePrefix="yes">Kadenz</Label>'
        f'<Label X="{X}" Y="76" Width="-20" Height="30" FontId="2" DisablePrefix="yes">{textes("Maintenance")}</Label>'
        + bouton("RepairButton", "Reparer", -228, condition='HideWhenDisabled="yes"')
        + bouton("UninstallButton", "Desinstaller", -124)
        + bouton("ModifyCancelButton", "Annuler", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Success">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("Installe")}'
        + selon_action(DESINSTALLATION, "Desinstalle")
        + selon_action(REPARATION, "Repare")
        + "</Label>"
        # Identité de Kadenz, puis le dossier choisi.
        f'<ImageControl X="{X}" Y="100" Width="64" Height="64" ImageFile="logo.png" />'
        f'<Label X="{X + 80}" Y="102" Width="-20" Height="34" FontId="1" DisablePrefix="yes">Kadenz</Label>'
        f'<Label X="{X + 80}" Y="138" Width="-20" Height="24" FontId="2" DisablePrefix="yes">{textes("Slogan")}</Label>'
        f'<Label X="{X}" Y="190" Width="-20" Height="20" FontId="3" DisablePrefix="yes" VisibleCondition={quoteattr(PAS_DESINSTALLATION)}>{textes("InstalleDans")}</Label>'
        f'<Label X="{X}" Y="212" Width="-20" Height="40" FontId="3" DisablePrefix="yes" VisibleCondition={quoteattr(PAS_DESINSTALLATION)}>[InstallFolder]</Label>'
        f'<Label X="{X}" Y="-90" Width="-20" Height="34" FontId="3" DisablePrefix="yes" VisibleCondition="WixStdBARestartRequired">{textes("RedemarrageRequis")}</Label>'
        # Case lue par l'extension au clic sur Terminer, qui lance alors Kadenz.
        f'<Checkbox Name="LancerKadenz" X="{X}" Y="-58" Width="-20" Height="24" TabStop="yes" FontId="3" VisibleCondition={quoteattr(PAS_DESINSTALLATION)}>{textes("Lancer")}</Checkbox>'
        + bouton("SuccessRestartButton", "Redemarrer", -124, largeur=140, condition='HideWhenDisabled="yes"')
        + bouton("TerminerButton", "Terminer", -20, action="<CloseWindowAction />")
        + "</Page>",
        f'<Page Name="Failure">{COTE}'
        f'<Label X="{X}" Y="24" Width="-20" Height="64" FontId="1" DisablePrefix="yes">{textes("Echec")}'
        + selon_action(DESINSTALLATION, "EchecDesinstallation")
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
