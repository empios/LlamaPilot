# Plan automatycznych aktualizacji LlamaPilot

Status: kod wdrożony lokalnie; publikacja i kwalifikacja aktualizacji zainstalowanych aplikacji pozostają do wykonania. Data: 2026-09-14.
Baza: `main`, commit `4ac3462`, wersja aplikacji `0.3.0`.

## Wykonanie

- Zaimplementowane: serwis Rust, weryfikacja podpisów przez Tauri, typowane IPC,
  panel Settings → Updates, harmonogram sprawdzania, opcjonalne pobieranie/instalacja,
  odliczanie restartu, odroczenie wersji, blokada pracy i niezapisanych edytorów.
- Dodana ochrona jednej instancji aplikacji i znacznik instalatora NSIS.
- Pipeline przygotowuje podpisane pakiety, metadane buildów i wspólny manifest;
  weryfikuje podpisy oraz zgodność hashy/wersji/targetów przed publikacją.
- Klucz publiczny jest wstrzykiwany przez overlay konfiguracji podczas release'u;
  zwykły lokalny build bez klucza pozostaje dostępny z ręcznym pobieraniem wydań.
- Stany oczekiwania i nieobsługiwanej instalacji są wyliczane z informacji o pracy,
  edytorach i rodzaju instalacji. Pobrany pakiet pozostaje w pamięci do zamknięcia aplikacji.
- Weryfikacja lokalna: TypeScript, 88 testów frontendowych i próg coverage, 8 testów
  publikacji (w tym rzeczywisty wektor Minisign), build Vite i kontrole Rust.
- Zbudowany podpisany NSIS z tymczasowym kluczem i osobnym testowym identifierem;
  podpis został niezależnie zweryfikowany. Instalator testowy nie był uruchamiany.
- Produkcyjny klucz i wymagane sekrety/zmienna GitHub Actions skonfigurowane 2026-09-14;
  sprawdzono podpis i zgodność klucza publicznego. Szczegóły w UPDATER_SETUP.md.
- Do wydania: integracja kodu do repozytorium, nowa wersja/tag,
  test upgrade'u między dwiema zainstalowanymi wersjami na każdej platformie,
  kwalifikacja UAC/uprawnień/notaryzacji i publikacja. Środowisko tej sesji to Windows;
  nie potwierdzono wykonania instalowanego upgrade'u na macOS ani Linux.

Instrukcja produkcyjnego uruchomienia: [RELEASING.md](RELEASING.md#updater-signing-and-first-release).
Instrukcja konfiguracji kluczy krok po kroku po polsku: [UPDATER_SETUP.md](UPDATER_SETUP.md).

## Cel i zakres

Aktualizacja zainstalowanej aplikacji LlamaPilot z poziomu aplikacji, z zachowaniem
ustawień, profili, modeli i runtime'ów. Aktualizowanie źródeł lub przebudowywanie
llama.cpp jest osobnym procesem i nie należy do tego mechanizmu.

Funkcja jest wykonalna w obecnej architekturze Tauri 2. Repozytorium ma już pipeline
Windows x64, macOS ARM64/x64 i Linux x64 oraz publikację kompletnego wydania przez
pojedynczy job. Brakuje integracji updatera, podpisów aktualizacji i manifestu.

## Proponowane zachowanie

- Domyślnie sprawdzanie aktualizacji w tle po uruchomieniu, następnie co 24 godziny,
  wyłącznie gdy aplikacja działa. Błąd sieci nie blokuje uruchomienia.
- Settings → Updates: bieżąca wersja, ostatnie sprawdzenie, nowa wersja, opis zmian,
  przyciski „Sprawdź teraz”, „Pobierz”, „Zainstaluj i uruchom ponownie”, „Później”.
- Ustawienia: automatyczne sprawdzanie (domyślnie włączone), pobieranie w tle
  (domyślnie wyłączone), automatyczna instalacja po zakończeniu pracy
  (domyślnie wyłączona). Ostatnia opcja obejmuje zgodę na restart aplikacji.
- Po włączeniu pełnej automatyzacji aplikacja pobiera aktualizację i instaluje ją
  po osiągnięciu bezczynności. Widoczne odliczanie z możliwością odroczenia pozwala
  zachować kontrolę. Bezczynność oznacza także brak niezapisanych edycji w UI.
- Pobieranie może działać równolegle z serwerem. Instalacja czeka, dopóki aktywny
  jest serwer, build, benchmark/auto-tune, pobieranie modelu lub inna operacja
  modyfikująca dane. Nie zatrzymuje ich automatycznie.
- Kanał stable; bez automatycznych downgrade'ów i bez powtarzania powiadomień
  o tej samej odroczonej wersji podczas jednej sesji.

## 1. Pakiety i podpisy

Użyć oficjalnego `tauri-plugin-updater`, włączyć `createUpdaterArtifacts` i umieścić
klucz publiczny w konfiguracji. Klucz prywatny oraz jego hasło przechowywać jako
sekrety CI (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`),
z kopią odzyskiwania poza repozytorium. Brak podpisu ma zatrzymywać release.
Podpis updatera jest wymagany; podpisy Windows Authenticode i Apple są odrębne.

Proponowana macierz pierwszego wdrożenia:

| Instalacja | Sposób aktualizacji |
| --- | --- |
| Windows x64 NSIS | Podpisany instalator NSIS; tryb `passive` |
| Windows MSI zarządzany administracyjnie | Informacja o wersji i ręczne wdrożenie MSI |
| macOS ARM64 / x64 | Podpisany pakiet aktualizacyjny `.app.tar.gz` właściwej architektury |
| Linux x64 AppImage | Podpisany AppImage; wymagana możliwość zapisu |
| Linux DEB | Informacja o wersji i aktualizacja pakietu przez administratora |
| Surowy EXE / build developerski | Brak automatycznej podmiany; odnośnik do instalatora |

Ograniczenie MSI jest decyzją produktu dla pierwszego wdrożenia: Tauri wspiera
również aktualizacje MSI. Nie przełączać istniejących instalacji MSI na NSIS.
Przed implementacją ustalić wiarygodny znacznik typu instalacji; sam system
operacyjny ani nazwa pliku wykonywalnego nie wystarczają. Nierozpoznana instalacja
otrzymuje tryb ręczny. Na macOS zweryfikować aplikację skopiowaną z DMG do Applications,
uprawnienia oraz oba warianty dystrybucji: ad-hoc i Developer ID/notaryzacja.

## 2. Wydawanie wersji

Zmienić `.github/workflows/release.yml` i `scripts/prepare-release.mjs`:

1. Każdy job buduje pakiety i podpisy, ale nie publikuje osobnego wydania.
2. Zbieranie artefaktów obejmuje także `.sig` i archiwa aplikacji macOS.
3. Końcowy job waliduje komplet pakietów, architektury, zgodność wersji z tagiem,
   podpisy i unikalność nazw. Archiwa obu architektur macOS wymagają różnych nazw.
4. Wygenerować jeden `latest.json` z wersją, opisem i adresami konkretnych assetów
   tego tagu; podpisy umieścić jako treść, nie odnośniki. Używać jawnego mapowania
   target → pakiet, niezależnego od kolejności plików w katalogu.
5. Zachować istniejącą publikację draft → upload wszystkich plików → publish.
   Niekompletne wydanie nie może zastąpić poprzedniego dostępnego wydania.

Proponowany endpoint:
`https://github.com/empios/LlamaPilot/releases/latest/download/latest.json`.
Publiczne pobieranie nie wymaga tokenu GitHub w aplikacji.

## 3. Backend i bezpieczeństwo wykonywanej pracy

- Dodać `src-tauri/src/updater/` i `src-tauri/src/commands/updater.rs`; zarejestrować
  plugin w `lib.rs`, serwis w `AppState`, komendy w `commands/mod.rs`.
- Udostępnić typowane komendy sprawdzania, pobierania, instalacji i odroczenia.
  Pobieranie, weryfikacja i instalacja pozostają w Rust; frontend nie przekazuje
  dowolnych URL-i ani ścieżek do instalatora.
- Stany: idle, checking, available, downloading, ready, waitingForIdle, installing,
  error, unsupported. Zdarzenia przenoszą postęp i przyczynę odroczenia do UI.
- Rozdzielić pobieranie od instalacji. Przed instalacją ponownie sprawdzić aktywność,
  zapisać stan i przejąć wspólną blokadę, która uniemożliwi rozpoczęcie nowej pracy
  między sprawdzeniem bezczynności a zamknięciem aplikacji.
- Objąć blokadą także operacje Git, skanowanie/zapisy katalogu, inspekcję runtime'u
  i zapisy profili. Obecne flagi supervisorów są punktem wyjścia, ale nie obejmują
  wszystkich operacji. Jawnie obsłużyć odmowę zamknięcia z niezapisanym formularzem.
- Timeout, ograniczone ponowienia i możliwość ponownego pobrania po błędzie.
  Niepoprawny podpis lub niezgodny target blokują instalację.
- Zachować identifier `com.llamacontrol.app` i obecne katalogi danych. Nowe pola
  ustawień mają wartości domyślne dla istniejących plików JSON. Przyszłe migracje
  danych wymagają kopii zapasowej i nie mogą usuwać danych potrzebnych starej wersji.

## 4. Frontend

Rozszerzyć `src/features/settings/settings-page.tsx` o panel Updates, dodać hook
obsługujący typowane IPC oraz niewielki wskaźnik dostępnej aktualizacji w app shell.
Rozszerzyć typy w `src/types/settings.ts` i model Rust w `config/settings.rs`.
Używać istniejących komponentów, wzorca błędów i obsługi postępu. Nieznany rozmiar
pobierania wymaga wskaźnika bez procentów. Tryb developerski nie odpytuje endpointu
automatycznie; testy używają kontrolowanego źródła aktualizacji.

## 5. Weryfikacja i kryteria odbioru

- Testy manifestu: brak targetu/podpisu, pomieszane architektury, kolizje nazw,
  niewłaściwa wersja i niekompletny release blokują publikację.
- Testy logiki: starsza/równa/nowsza wersja, offline, uszkodzony plik, błędny podpis,
  ponowienie, odroczenie i ustawienia odczytane ze starego pliku.
- Testy współbieżności: start serwera/builda podczas próby instalacji; instalacja
  nie przechodzi, gdy trwa praca, pobieranie modelu albo niezapisana edycja.
- Test UI: sprawdzenie → pobieranie → odroczenie → instalacja; błędy i brak rozmiaru.
- Rzeczywisty upgrade dwóch podpisanych wersji na Windows NSIS, obu macOS i Linux
  AppImage. Sprawdzić numer wersji po restarcie oraz zachowanie danych i profili.
- Windows: przetestować instalację użytkownika, wymaganie podniesienia uprawnień
  i odmowę UAC. Sprawdzić ręczny fallback dla MSI, DEB i surowego EXE.
- Uruchomić istniejące kontrole wersji, TypeScript, Vitest, build oraz Rust
  fmt/clippy/test zgodnie z CI. Sam poprawny build nie kwalifikuje updatera.

## 6. Wprowadzenie do użycia i odzyskiwanie

Pierwszą wersję zawierającą updater użytkownik instaluje ręcznie. Dopiero kolejne
wydanie może zaktualizować ją automatycznie; obecna wersja nie ma kodu updatera.
Najpierw zweryfikować parę wersji na kanale testowym, potem opublikować stable.
Zachować dostęp do poprzednich instalatorów. Nie obiecywać automatycznego rollbacku:
wadliwe wydanie naprawiać nową, wyższą wersją; ewentualny ręczny downgrade wymaga
sprawdzenia kompatybilności danych. Udokumentować odzyskiwanie po przerwaniu instalacji
i procedurę zmiany klucza podpisującego przed utratą starego klucza.

Kolejność prac: pipeline i podpisy → backend z blokadą pracy → panel i automatyzacja
→ rzeczywiste testy aktualizacji → pierwsze wydanie z updaterem. Warunki startu
publikacji: dostęp do sekretów GitHub Actions i środowisk testowych wszystkich
platform, które zostaną ogłoszone jako obsługiwane.

## Źródła techniczne

- [Tauri Updater](https://v2.tauri.app/plugin/updater/) — API, podpisy i formaty pakietów.
- [Tauri Action](https://github.com/tauri-apps/tauri-action) — integracja z GitHub Actions.
- Podstawa decyzji projektowych: aktualny kod `AppState`, supervisorów, ustawień,
  konfiguracje `tauri.*.conf.json` i istniejący pipeline publikacji repozytorium.
