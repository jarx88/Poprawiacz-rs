# UX hill climb: PoprawiaczTekstu

Cel właściciela (2026-11-18): poprawić czytelność i funkcjonowanie UI, pętla do oceny 9.

## Ekrany i stany (stałe dane, DPR 1, ciemny motyw, font Selawik = odpowiednik Segoe UI)

| Plik | Stan | Okno |
|---|---|---|
| `idle.png` | po uruchomieniu, brak sesji | 1100×760 |
| `loading.png` | 4 panele w toku | 1100×760 |
| `mixed.png` | 1 strumieniuje, 2 gotowe, 1 w toku | 1100×760 |
| `done.png` | 4 gotowe, podkreślone zmiany | 1100×760 |
| `errors.png` | błąd API, anulowano, 2 gotowe, komunikat po 👍 | 1100×760 |
| `menu.png` | otwarte menu akcji panelu | 1100×760 |
| `settings.png` | okno ustawień | 1100×760 |
| `original.png` | okno „Oryginalny tekst” | 1100×760 |
| `long.png` | długi tekst w minimalnym oknie | 720×520 |

Odtworzenie: `node ux-climb/capture.mjs <runda>` → `ux-climb/round-<runda>/` (+ `metrics.json`).
Dwa przebiegi z rzędu: 7–9 z 9 plików identycznych bajt po bajcie, reszta różni się 7 pikselami
(wygładzanie animacji). Sędziowie dostają kopię dziewięciu zrzutów rundy w `ux-climb/shots/<losowa
nazwa>/` (nazwa nie zdradza numeru rundy) i brief z `judge-brief.md`.

Poza dziewięcioma ocenianymi stanami harness robi trzy kontrolne: `settings-end` i `long-end`
(przewinięte na dół: wygaszenie ma zniknąć) oraz `done-keys` (naciska 2 i Ctrl+Enter; przebieg
kończy się błędem, jeśli skróty nie wywołają wklejenia i korekty).

W repo są tylko skrypty, plan, brief i oceny. Lokalne pozostają zrzuty i fonty:

```sh
mkdir -p ux-climb/fonts ux-climb/ref
curl -sSL -o /tmp/selawik.zip https://github.com/microsoft/Selawik/releases/download/1.01/Selawik_Release.zip
unzip -o -j /tmp/selawik.zip '*.woff2' -d ux-climb/fonts
cp docs/4pieaAtfDO.png ux-climb/ref/stary-python-gotowe.png
cp docs/sR3R6IsuQv.png ux-climb/ref/stary-python-w-toku.png
```

Bez fontów harness działa, ale renderuje się w foncie zastępczym i zrzuty nie są porównywalne.

## Referencje

- `ref/stary-python-*.png`: poprzednia aplikacja (CustomTkinter), którą ta zastępuje.
- Brak zrzutu konkurenta z tą samą treścią (wymagałby logowania do cudzej usługi): luka.

## Twarde ograniczenia (zachować, nawet jeśli sędzia chce inaczej)

1. Tylko ciemny motyw; okno bez ramki z własnym paskiem tytułu; ✕ chowa do zasobnika.
2. Cztery panele dostawców w siatce 2×2, każdy w kolorze swojego dostawcy.
3. Przepływ: Ctrl+Shift+C → 4 wyniki → „Użyj …” wkleja. Żadna akcja nie znika: zakładki stylów,
   Oryginał, Anuluj, Minimalizuj, Popraw, 👍, menu ⚙, anulowanie panelu ✖, Ponów.
4. Zmienione słowa podkreślone kolorem panelu (nie tłem).
5. Wersja i czas buildu w pasku tytułu; licznik „sesja N · api X/4”.
6. Interfejs po polsku; bez nowych zależności; zmiany tylko we froncie (`src/`).

## Próg i budżet

- Próg: każdy sędzia ≥ 9 w każdej kategorii i nasza aplikacja ≥ referencja.
- Limit: 5 rund; stop także po 2 rundach bez zysku powyżej szumu (±0,5–1).
- Sędziowie: Claude Sonnet (tani, każda runda) i Codex/GPT (inna rodzina; runda 0 i po
  przekroczeniu progu u pierwszego sędziego).
- Bramka programowa: `metrics.json` (kontrast WCAG tekstu, ucięta treść, przepełnienie strony),
  `pnpm exec tsc --noEmit`, `pnpm test`.

## Wynik wspinaczki 2026-11-18 (5 rund, limit wyczerpany, próg 9 nieosiągnięty)

Średnia z 7 kategorii, nasza aplikacja (szczegóły: `scores.tsv`):

| Runda | Claude Sonnet | Codex GPT |
|---|---|---|
| 0 | 7,0 | 7,4 |
| 1 | 7,4 | – |
| 2 | 7,8 | – |
| 3 | 7,9 | – |
| 4 | 8,3 | 8,6 |
| 5 | 8,2 | 8,6 |
| 6 | 8,3 | 8,7 |

Runda 6 (na prośbę właściciela, ponad limit): drobiazgi z ostatniego werdyktu Codexa, bez zysku
ponad szum. Plateau: Claude 8,2–8,3 od rundy 4, Codex 8,6–8,7.

Ustalenia dla sędziów dopisane w trakcie: punkty 7–12 w `judge-brief.md`.
Po wspinaczce, na prośbę właściciela, doszły nowe zachowania (bez oceny sędziów): „Kopiuj tekst”
w menu panelu, klawisze 1–4 dla „Użyj”, Ctrl+Enter dla „Popraw”, „Ustawienia” w panelu z błędem
klucza.
