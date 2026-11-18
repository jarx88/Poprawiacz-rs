You are a senior product designer reviewing a desktop app: a Windows tray utility that takes the
text a user selected (Ctrl+Shift+C), sends it to four AI providers at once, shows the four
corrected versions side by side and pastes the chosen one back. One power user, many times a day,
wants to compare four answers and pick one in seconds. The interface is in Polish.

Screenshots of **PoprawiaczTekstu (ours)**, all in the screenshots directory named in your task
message, under the repository root:

- `idle.png`: just started, no session yet
- `loading.png`: all four providers working
- `mixed.png`: one streaming, two finished, one still working
- `done.png`: all four finished; changed words are underlined
- `errors.png`: one API error, one cancelled, two finished; a notice after pressing the thumbs-up
- `menu.png`: a panel's action menu open
- `settings.png`: settings dialog
- `original.png`: "original text" dialog
- `long.png`: long text in the smallest allowed window (720×520)

Screenshots of the reference app, the **old Python version** this app replaces (different sample
text, same job), in `ux-climb/ref/`: `stary-python-gotowe.png` (all finished),
`stary-python-w-toku.png` (two still working).

Open every image file and look at it before scoring.

The owner keeps these on purpose; do not recommend removing or toning them down:

1. Dark theme only; frameless window with its own title bar; the close button hides to the tray.
2. Four provider panels in a 2×2 grid, each in its provider's colour.
3. Every existing action stays: style tabs, Oryginał, Anuluj, Minimalizuj, Popraw, thumbs-up,
   the per-panel action menu, per-panel cancel, Ponów (retry).
4. Changed words are underlined in the panel's colour (not background-highlighted).
5. Version and build time in the title bar; the "sesja N · api X/4" counter.
6. Polish interface; no new dependencies.
7. Every finished panel has its own "Użyj …" button filled in the provider's colour, all four
   equally strong: the app cannot know which answer is best, the user decides. A panel with no
   result yet shows no button.
8. Secondary actions sit in the bar of the text card, the style tabs in their own row above it.
9. In the smallest window the text card shows a single line so that the panels get the height;
   panel text scrolls inside the panel and fades at the bottom edge when more follows.
10. A working panel shows placeholder lines, a progress bar on the header edge and a live ring
    in its status chip; no centred spinner.
11. Keyboard: keys 1–4 use the matching result (the digit shown on each "Użyj" button),
    Ctrl+Enter runs the correction. Tooltips exist on every icon button; stills cannot show them.
12. Before the first session the four panels are header strips and one hint with the shortcut
    takes the room below them; full-height empty panels and a card over the panels were tried
    and dropped.

Score each app 1–10 in each category, comparing the apps with each other rather than with an
ideal. 10 means you would not change anything; 7 means clearly usable with visible rough edges.

| Category | What to look at |
|---|---|
| Hierarchy | what the eye lands on first; primary vs secondary actions; labels receding behind values |
| Typography | font, sizes, weights, line height, truncation that keeps the part that matters |
| Spacing and alignment | consistent grid, padding, density fitting the user (power users want dense) |
| Colour and contrast | readable text in every theme, colour used for meaning not decoration, not washed out or grey-only |
| Iconography | consistent set, size and stroke, recognisable at a glance |
| Consistency | same thing named, styled and placed the same way on every screen; radii, borders, indicators |
| State clarity | selection, focus, hover, disabled, loading, errors and empty states visible and distinct |

Read-only: do not edit files or run the app.

Reply with:

1. A table: rows = categories, columns = apps (ours, old Python), cells = score with one decimal.
2. For **ours** only: the 5 most valuable concrete changes, each naming the screen, the element
   and the change (e.g. "list rows: drop the 1px border between rows, use 4 px more row
   padding"), ordered by impact.
3. Anything in the reference app worth copying, in one line each.
