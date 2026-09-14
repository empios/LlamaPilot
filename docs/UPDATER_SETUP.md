# Uruchomienie aktualizacji automatycznych

## Stan konfiguracji — 2026-09-14

Konfiguracja produkcyjna została wykonana dla `empios/LlamaPilot`:

- Sekrety Actions: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Zmienna Actions: `TAURI_UPDATER_PUBLIC_KEY`.
- Lokalny klucz: `%USERPROFILE%\.tauri\LlamaPilot\llamapilot-production.key`.
- Klucz publiczny: ten sam plik z końcówką `.pub`.
- Hasło: `%USERPROFILE%\.tauri\LlamaPilot\password.dpapi`, zaszyfrowane przez Windows
  dla bieżącego konta. Katalog ma ograniczone uprawnienia do właściciela i SYSTEM.
- Sprawdzono podpis kluczem produkcyjnym i zgodność klucza publicznego zapisanego na GitHub.

**Nie generuj kolejnego klucza przy następnych wydaniach.** Poniższe kroki generowania
służą jako instrukcja odtworzenia konfiguracji, a nie polecenie jej ponowienia.
Kod updatera nadal wymaga integracji do repozytorium i wydania nowej wersji.

Przed migracją lub reinstalacją Windows zachowaj kopię zaszyfrowanego klucza prywatnego
i zapisz jego hasło w menedżerze haseł. Sam plik `password.dpapi` nie jest przenośną kopią
hasła — wymaga oryginalnego konta i środowiska Windows. Pomocniczy skrypt
`%USERPROFILE%\.tauri\LlamaPilot\Copy-SigningPassword.ps1` pozwala skopiować hasło
do schowka bez wyświetlania go w konsoli; uruchom go tylko podczas zapisywania hasła
w swoim menedżerze haseł. Prywatne dane nie są częścią repozytorium.

Kod updatera i pipeline są przygotowane. Produkcyjny klucz jest już skonfigurowany.
Tymczasowy klucz użyty do testu instalatora został usunięty i nie
nadaje się do publikacji. Oficjalnego wydania 0.3.0 nie można zaktualizować automatycznie,
ponieważ nie zawiera jeszcze updatera. Lokalne instalatory zbudowane z kodu tego PR
zawierają updater, ale wymagają opublikowanego manifestu do sprawdzania aktualizacji.

## 1. Wygeneruj klucz produkcyjny

W PowerShell, z katalogu projektu:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.tauri" | Out-Null
npm run tauri signer generate -- -w "$env:USERPROFILE\.tauri\llamapilot-production.key"
```

Ustaw hasło w interaktywnym pytaniu narzędzia. Powstaną pliki:

- `llamapilot-production.key` — klucz prywatny, tylko dla procesu podpisywania.
- `llamapilot-production.key.pub` — klucz publiczny, dołączany do aplikacji.

Zachowaj bezpieczną kopię klucza prywatnego i hasła poza repozytorium. Nie generuj
nowego klucza przy każdym wydaniu. Nie umieszczaj klucza prywatnego w commicie,
komentarzu, rozmowie ani logu.

## 2. Ustaw GitHub Actions

W repozytorium `empios/LlamaPilot` otwórz **Settings → Secrets and variables → Actions**.

| Miejsce | Nazwa | Wartość |
| --- | --- | --- |
| Variables → New repository variable | `TAURI_UPDATER_PUBLIC_KEY` | Cała zawartość pliku `.key.pub` |
| Secrets → New repository secret | `TAURI_SIGNING_PRIVATE_KEY` | Cała zawartość pliku `.key` |
| Secrets → New repository secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Hasło wybrane przy generowaniu klucza |

Wpisuj zawartość plików, nie ścieżki z własnego komputera. Nie dekoduj formatu
base64 generowanego przez Tauri. Klucz prywatny pozostaje sekretem CI; do aplikacji
trafia tylko klucz publiczny. To osobne podpisy od Windows Authenticode i Apple
Developer ID/notaryzacji.

## 3. Przygotuj pierwsze wydanie

1. Wybierz nową wersję, np. `0.4.0`, i zsynchronizuj `package.json`, `package-lock.json`,
   `src-tauri/Cargo.toml`, wpis aplikacji w `src-tauri/Cargo.lock` i `src-tauri/tauri.conf.json`.
2. Uruchom kontrole projektu, w tym `npm run check:version` i `npm run test:release`.
3. Przetestuj upgrade pomiędzy dwiema podpisanymi wersjami na oddzielnym identyfikatorze
   aplikacji i testowym endpointcie HTTPS. Lista scenariuszy jest w
   [RELEASING.md](RELEASING.md#qualification-before-publication).
4. Po przeglądzie i integracji zmian do `main` utwórz i wypchnij odpowiadający wersji tag.
   Dla przygotowanej wersji `0.4.0`:

   ```powershell
   git tag -a v0.4.0 -m "LlamaPilot v0.4.0"
   git push origin v0.4.0
   ```

5. Workflow **Release desktop installers** zbuduje wszystkie platformy, podpisze pakiety,
   zweryfikuje ich kompletność i podpisy, doda `latest.json` oraz opublikuje całe wydanie.
   Brak klucza lub błąd któregokolwiek pakietu zatrzyma publikację.
6. Pierwsze wydanie z updaterem zainstaluj ręcznie. Następne wydanie o wyższym numerze
   będzie dostępne w **Settings → Updates**.

Automatyczne sprawdzanie jest domyślnie włączone. Pobieranie w tle i instalację
z restartem włącza się osobno. Instalacja czeka na zakończenie aktywnej pracy
i zapisanie/zamknięcie edytorów; pokazuje odliczanie 30 sekund oraz przycisk **Later**.

## Co już sprawdzono lokalnie

### Brak manifestu aktualizacji

Opublikowane wcześniej wydanie `v0.3.0` nie zawiera `latest.json`; adres
`releases/latest/download/latest.json` zwraca wtedy HTTP 404. Samo skonfigurowanie
kluczy i zbudowanie lokalnego instalatora nie publikuje tego pliku. Aplikacja pokazuje
**Update service unavailable** i pozwala ponowić sprawdzenie lub otworzyć wydania.
Nie oznacza to udanego sprawdzenia ani potwierdzenia aktualności aplikacji.
Należy opublikować nowe wydanie opisane powyżej, z manifestem i podpisanymi pakietami.
Ten sam stan może oznaczać inną nieudaną odpowiedź HTTP serwera; błędy sieci,
niepoprawnego JSON i podpisu nadal są zgłaszane jako błędy.

### Walidacja lokalna

- Testy TypeScript/React, manifestu i podpisów oraz kontrole Rust.
- Budowę podpisanego instalatora NSIS z odrębną testową nazwą/identyfikatorem
  i niezależną weryfikację jego podpisu.

Nie wykonano instalowanego upgrade'u między dwiema wersjami ani kwalifikacji macOS,
Linux i zachowania Windows UAC. Testowy instalator nie był uruchamiany. Te kontrole
pozostają warunkiem deklarowania gotowości wydania produkcyjnego na danej platformie.

MSI, DEB i samodzielny EXE używają ręcznego pobierania pakietów. Aktualizacje wewnątrz
aplikacji obejmują oznaczoną instalację NSIS, zainstalowaną aplikację macOS i zapisywalny
AppImage. Modele, runtime'y i profile pozostają w dotychczasowych katalogach.

Procedury naprawy przerwanej instalacji i zmiany klucza opisuje
[RELEASING.md](RELEASING.md#recovery).
