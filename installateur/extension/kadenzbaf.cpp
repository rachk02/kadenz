// Extension (BAFunctions) de l'assistant WixStdBA de KadenzSetup.
//
// WixStdBA ne sait ni changer une variable depuis un bouton ni lancer une
// application selon une case à cocher. Cette extension ajoute ces deux gestes :
// - SuivantButton et PrecedentButton passent KadenzEtape à 2 ou à 1 ; leur
//   ChangePageAction, exécutée ensuite par le thème, réaffiche la page Install,
//   qui montre l'étape correspondante (langue, puis dossier et installation) ;
// - TerminerButton lance Kadenz si la case LancerKadenz est visible et cochée,
//   puis le thème ferme la fenêtre (CloseWindowAction). Le lancement part du
//   processus de l'assistant, qui tourne sans élévation, comme le bouton
//   « Lancer » de WixStdBA, et vise la même variable LaunchTarget.
//
// Construit par installateur\construire.ps1 (MSVC, en-têtes et bibliothèques
// des paquets NuGet WiX 5.0.1).

#include <windows.h>
#include <objbase.h>
#include <shlwapi.h>
#include <strsafe.h>
#include <CommCtrl.h>

#include "dutil.h"
#include "dictutil.h"
#include "strutil.h"
#include "shelutil.h"

#include "BootstrapperApplicationBase.h"
#include "BAFunctions.h"
#include "IBAFunctions.h"
#include "BalBaseBAFunctions.h"
#include "BalBaseBAFunctionsProc.h"

static const WORD ID_SUIVANT = BAFUNCTIONS_FIRST_ASSIGN_CONTROL_ID;
static const WORD ID_PRECEDENT = BAFUNCTIONS_FIRST_ASSIGN_CONTROL_ID + 1;
static const WORD ID_TERMINER = BAFUNCTIONS_FIRST_ASSIGN_CONTROL_ID + 2;
static const WORD ID_LANCER = BAFUNCTIONS_FIRST_ASSIGN_CONTROL_ID + 3;

static HINSTANCE vhInstance = NULL;

class CKadenzBAFunctions : public CBalBaseBAFunctions
{
public:
    virtual STDMETHODIMP OnThemeControlLoading(
        __in LPCWSTR wzName,
        __inout BOOL* pfProcessed,
        __inout WORD* pwId,
        __inout DWORD* /*pdwAutomaticBehaviorType*/
        )
    {
        // Le comportement automatique (actions, conditions, variable liée)
        // reste celui du thème : seul l'identifiant est fixé.
        struct { LPCWSTR wzNom; WORD wId; } rgControles[] =
        {
            { L"SuivantButton", ID_SUIVANT },
            { L"PrecedentButton", ID_PRECEDENT },
            { L"TerminerButton", ID_TERMINER },
            { L"LancerKadenz", ID_LANCER },
        };

        for (DWORD i = 0; wzName && i < countof(rgControles); ++i)
        {
            if (CSTR_EQUAL == ::CompareStringW(LOCALE_INVARIANT, 0, wzName, -1, rgControles[i].wzNom, -1))
            {
                *pwId = rgControles[i].wId;
                *pfProcessed = TRUE;
                break;
            }
        }

        return S_OK;
    }

    // Appelée avant le traitement du thème : *pfProcessed reste FALSE pour que
    // l'action du bouton (ChangePageAction, CloseWindowAction) s'exécute ensuite.
    virtual STDMETHODIMP WndProc(
        __in HWND hWnd,
        __in UINT uMsg,
        __in WPARAM wParam,
        __in LPARAM /*lParam*/,
        __inout BOOL* /*pfProcessed*/,
        __inout LRESULT* /*plResult*/
        )
    {
        if (WM_COMMAND != uMsg || BN_CLICKED != HIWORD(wParam))
        {
            return S_OK;
        }

        switch (LOWORD(wParam))
        {
        case ID_SUIVANT:
            m_pEngine->SetVariableNumeric(L"KadenzEtape", 2);
            break;

        case ID_PRECEDENT:
            m_pEngine->SetVariableNumeric(L"KadenzEtape", 1);
            break;

        case ID_TERMINER:
            LancerSiCoche(hWnd);
            break;
        }

        return S_OK;
    }

private:
    void LancerSiCoche(
        __in HWND hWnd
        )
    {
        HRESULT hr = S_OK;
        LPWSTR sczCible = NULL;
        LPWSTR sczCibleFormatee = NULL;
        LPWSTR sczDossier = NULL;
        LPWSTR sczDossierFormate = NULL;
        HWND hwndCase = ::GetDlgItem(hWnd, ID_LANCER);

        if (!hwndCase || !::IsWindowVisible(hwndCase) || BST_CHECKED != ::SendMessageW(hwndCase, BM_GETCHECK, 0, 0))
        {
            ExitFunction();
        }

        hr = BalGetStringVariable(L"LaunchTarget", &sczCible);
        BalExitOnFailure(hr, "Kadenz : variable LaunchTarget introuvable.");

        hr = BalFormatString(sczCible, &sczCibleFormatee);
        BalExitOnFailure(hr, "Kadenz : mise en forme de LaunchTarget impossible.");

        if (SUCCEEDED(BalGetStringVariable(L"LaunchWorkingFolder", &sczDossier)))
        {
            BalFormatString(sczDossier, &sczDossierFormate);
        }

        hr = ShelExec(sczCibleFormatee, NULL, L"open", sczDossierFormate, SW_SHOWDEFAULT, hWnd, NULL);
        BalExitOnFailure(hr, "Kadenz : lancement impossible de %ls", sczCibleFormatee);

    LExit:
        ReleaseStr(sczDossierFormate);
        ReleaseStr(sczDossier);
        ReleaseStr(sczCibleFormatee);
        ReleaseStr(sczCible);
    }

public:
    CKadenzBAFunctions(
        __in HMODULE hModule
        ) : CBalBaseBAFunctions(hModule)
    {
    }
};

extern "C" BOOL WINAPI DllMain(
    __in HINSTANCE hInstance,
    __in DWORD dwReason,
    __in LPVOID /*pvReserved*/
    )
{
    switch (dwReason)
    {
    case DLL_PROCESS_ATTACH:
        ::DisableThreadLibraryCalls(hInstance);
        vhInstance = hInstance;
        break;

    case DLL_PROCESS_DETACH:
        vhInstance = NULL;
        break;
    }

    return TRUE;
}

extern "C" HRESULT WINAPI BAFunctionsCreate(
    __in const BA_FUNCTIONS_CREATE_ARGS* pArgs,
    __inout BA_FUNCTIONS_CREATE_RESULTS* pResults
    )
{
    HRESULT hr = S_OK;
    CKadenzBAFunctions* pBAFunctions = NULL;

    BalInitialize(pArgs->pEngine);

    pBAFunctions = new CKadenzBAFunctions(vhInstance);
    ExitOnNull(pBAFunctions, hr, E_OUTOFMEMORY, "Kadenz : création de l'extension impossible.");

    hr = pBAFunctions->OnCreate(pArgs->pEngine, pArgs->pCommand);
    ExitOnFailure(hr, "Kadenz : OnCreate de l'extension en échec.");

    pResults->pfnBAFunctionsProc = BalBaseBAFunctionsProc;
    pResults->pvBAFunctionsProcContext = pBAFunctions;
    pBAFunctions = NULL;

LExit:
    ReleaseObject(pBAFunctions);
    return hr;
}

extern "C" void WINAPI BAFunctionsDestroy(
    __in const BA_FUNCTIONS_DESTROY_ARGS* /*pArgs*/,
    __inout BA_FUNCTIONS_DESTROY_RESULTS* /*pResults*/
    )
{
    BalUninitialize();
}
