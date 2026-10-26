# Flightdeck — a fast, keyboard-first cockpit for Camunda 7

*Design document. Read this before touching the UI code; every screen, shortcut and
colour below is a decision, not an accident. If you change a decision, change it here first.*

---

## 0. One-paragraph summary

Flightdeck is a native Rust desktop application (egui/eframe, wgpu) that replaces Camunda
Cockpit for day-to-day operations on a Camunda 7 engine: browsing definitions, inspecting
running and finished process instances on the BPMN diagram, reading every parameter of
every BPMN element, searching instances by id / business key / variable / activity /
incident, searching elements inside a diagram, and acting on instances (variables,
incidents, jobs, tokens, batches). The diagram is the primary surface. The camera is
unlimited and smooth. Everything is reachable with the mouse, the keyboard, or both, through
one uniform command system.

---

## 1. Why: an honest inventory of Cockpit's shortcomings

Observed on Camunda Run 7.19 with the user's real diagrams (≈800 sequence flows, ≈240
gateways, 10 lanes, 40 timers, boundary events, call activities, 0.8–1.1 MB of XML each).

| # | Cockpit problem | Consequence | Flightdeck requirement |
|---|---|---|---|
| 1 | Zoom clamped (≈0.2×–4×), wheel scrolls instead of zooming, Ctrl+wheel zooms in big jumps | Big diagrams are unreadable at min zoom and you can't get close to a dense cluster | Zoom range 0.005×–200×, zoom-to-cursor, smooth exponential animation, pinch support, keyboard zoom |
| 2 | Panning only by drag; no keyboard; jerky | Hands leave the keyboard constantly | WASD/arrows/HJKL pan with acceleration, inertial drag, fit/zoom shortcuts, minimap |
| 3 | No element properties: clicking a service task shows nothing about `delegateExpression`, listeners, I/O mappings, timer expressions, conditions | You open the Modeler or the raw XML to know what a block does | Element inspector showing *every* attribute and extension element, plus live runtime stats for that element |
| 4 | No way to find an element in the diagram | You scroll around a 240-gateway diagram hunting for `Gateway_1x7f` | Ctrl+F in-diagram search by id / name / type / implementation / expression text, with dimming, cycling and camera jumps |
| 5 | Instance search is a clunky filter widget; cannot search finished instances by variable value; cannot show variable values as columns; cannot search by "passed through activity" | Support questions ("which instance has contract 12345?") take minutes | One search language with autocomplete, runtime + history modes, variable columns, executed-activity criterion, saved searches |
| 6 | Only one instance open at a time; browser back is the only history | Comparing two instances means two browser tabs that each reload everything | In-app tabs with per-tab back/forward history |
| 7 | Variables tab: truncated values, JSON unreadable, no search inside values | Copy to an editor to read a payload | Variable viewer with JSON/XML tree, in-value search, typed editing, copy-on-click |
| 8 | Incident stack traces in a tiny popup | Horizontal scrolling misery | Resizable trace pane with search and copy |
| 9 | Token modification, batch ops, history timeline are Enterprise-only | Community users use the REST API by hand | Modification (move/cancel/start tokens), batch suspend/delete/retry/set-variables, activity timeline — all on the open REST API |
| 10 | Timers: you cannot see *when* a waiting timer fires without opening Jobs | Guesswork | Timer badges with countdown; job due-date editing in place |
| 11 | Everything needs the mouse; nothing has a shortcut | Slow | Every action is a Command with a name, a shortcut and a palette entry |
| 12 | Data goes stale silently | Acting on stale state | Auto-refresh with a visible age indicator, pause, manual refresh |

Things that are **not** goals: editing BPMN, Tasklist/forms, Admin (users, groups,
authorizations), DMN/CMMN viewers (DMN table viewer is a later candidate), Camunda 8
(different API; the client crate is isolated so a second backend is possible later).

---

## 2. Design principles

1. **One command system.** Every user action (navigate, zoom, retry job, delete instance…)
   is a `Command` with a stable id, a human name, an optional default shortcut, an
   `enabled(&State) -> bool` predicate and an executor. Buttons, context menus, the command
   palette, shortcuts and tooltips all read from the same registry. This is what makes the UI
   uniform: there is no action that exists only as a button somewhere.
2. **Diagram first, lists second, both linked.** Selecting an element filters the list;
   selecting a row highlights the element; hovering either highlights the other. Nothing is
   "a table in another tab you must correlate by id in your head".
3. **Keyboard and mouse are peers.** Anything the mouse can do has a key, anything a key can
   do has a visible control or context-menu item showing that key. No hover-only affordances.
4. **Never block, never lie about freshness.** All I/O is asynchronous; each data block shows
   its age; a request in flight is a thin progress line, never a modal spinner. Stale data is
   displayed (dimmed) while refreshing, errors keep the last good data visible.
5. **Progressive disclosure, not hiding.** Dense information is organised in collapsible
   sections with stable order; collapsed state is remembered. Nothing requires a dialog unless
   it is destructive, and then the confirmation is inline and states the exact effect
   ("Delete 1 instance of `order_process` v3?").
6. **Few, consistent primitives.** Exactly four typographic sizes, one spacing grid (4/8 px),
   two corner radii, one accent colour, a fixed semantic palette. A new screen must be
   composable from existing primitives (toolbar, table, inspector section, key/value row,
   badge, chip, toast).
7. **Smoothness is a feature.** The camera, panel resizing, selection highlights and list
   scrolling animate with short, physically plausible easing (≤150 ms, exponential), never
   with bounces or long transitions. Animations never delay input: input always targets the
   *target* state, not the animated one.
8. **Escape always backs out one level.** Overlay → search → selection → nothing.
   Predictable and discoverable.

---

## 3. Technology decision

| Option | Verdict |
|---|---|
| **egui + eframe (wgpu)** | **Chosen.** Immediate mode makes custom 2-D painting (the diagram) first class; 60 fps with thousands of shapes is routine; keyboard handling is simple and global; native on Linux/Windows/macOS and compiles to wasm for a browser build later; AccessKit gives screen-reader support for free. |
| iced | Good async story, but the `canvas` widget is less ergonomic for a heavily interactive scene with overlays, and layout of complex docks is slower to iterate. |
| Slint | Declarative UI is nice for forms, weak for a custom scene graph with thousands of hit-testable shapes. |
| Dioxus / Leptos / Tauri | The diagram would end up in JS canvas or SVG again — the exact thing we're escaping; webview input latency and wheel handling are what makes Cockpit feel clunky. |

Supporting crates: `roxmltree` (BPMN XML), `reqwest` + `tokio` (REST, on a background
runtime thread), `serde_json`, `egui_extras` (tables), `egui_kittest` (headless UI tests),
`directories` + `toml` (settings).

Fonts: Noto Sans (system, Cyrillic coverage is mandatory — the user's diagrams are Russian)
and DejaVu Sans Mono, loaded at start-up when found, with egui's built-in fonts as fallback.

---

## 4. Information architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Top bar:  [engine ● 42 ms]  [ tab | tab | tab | + ]        [⌕ Ctrl+K]  [⚙] [?]│
├──────────────────────────────────────────────────────────────────────────────┤
│ Place toolbar: breadcrumb ▸ ▸ ▸     place-specific controls      age ⟳ ‖     │
├────────────────────────────────────────────────────────┬─────────────────────┤
│                                                        │ Dock (tabs):        │
│                      Main area                         │  Inspector          │
│          (diagram, or a table for list places)         │  Instances          │
│                                                        │  Variables …        │
│  [find bar when Ctrl+F]                   [minimap]    │  (resizable, Ctrl+B)│
├────────────────────────────────────────────────────────┴─────────────────────┤
│ Status line: selection · zoom 137% · 1 req in flight · hints for current mode │
└──────────────────────────────────────────────────────────────────────────────┘
                                                        toasts stack bottom-right
```

### 4.1 Places (what a tab can show)

| Place | Main area | Dock tabs |
|---|---|---|
| **Definitions** (home) | Table: key, name, latest version, running instances, incidents, suspended; filter box; group-by-key with version expansion | — (dock hidden) |
| **Definition** `{id}` | Diagram with per-activity running counts and incident badges | Inspector · Instances (of this version / all versions) · Incidents · Jobs · Versions · Called definitions |
| **Instance** `{id}` | Diagram with live tokens, historic path, incidents; ancestry breadcrumb (super-instances) | Overview · Inspector · Variables · Tasks · Incidents · Jobs · External tasks · Called instances · Timeline · Modify |
| **Search** `{query}` | Query bar + results table with configurable columns (incl. variable columns) | Row preview (variables of the focused row) |
| **Incidents** | Global incident table | Trace |
| **Batches** | Running/finished batches with progress | Failed jobs of selected batch |
| **Settings** | Engines, refresh, input, theme, confirmations, external XML editor | — |
| **File** `{path}` | Offline diagram of a local `.bpmn` (Ctrl+O, drag and drop, `flightdeck file.bpmn`) | Inspector · Matches |
| **XML** `{document}` | The document's source with line numbers and syntax colours, scrolled to and highlighting an element; or a git-style line diff of two documents | — |
| **Diff** `{before, after}` | One diagram with every change marked (or before / after / side by side) | Changes · Inspector |

Tabs behave like a browser: each tab has its own history (Alt+←/→, mouse back/forward),
`Ctrl+T` new tab (Definitions), `Ctrl+W` close, `Ctrl+Tab`/`Ctrl+Shift+Tab` cycle,
`Ctrl+1..9` jump, middle-click / `Shift+Enter` on any link opens it in a new tab.
Tabs show a short title ("order_process v3", "inst …f3a2", "search: var.amount>1000") and a
dot when their data has pending refresh errors.

### 4.2 The command palette (`Ctrl+K`, also `Ctrl+P`)

A single input that returns mixed results, grouped:

- **Commands** matching the typed words (fuzzy), with shortcut hints; only commands
  currently enabled are shown (disabled ones are listed greyed at the bottom with the reason).
- **Go to**: process definitions by key/name; instance id (any 36-char uuid pasted); business
  key; tabs.
- **Search**: the current text as an instance search ("Search instances for `…` ↵").

Prefix shortcuts for power users (not required knowledge): `>` commands only, `#` instance
id / business key, `@` elements of the current diagram, `?` help.

---

## 5. The diagram viewport

### 5.1 Camera model

```
Camera { center: world Vec2, zoom: f32 }            // 1.0 = 1 BPMN unit per logical pixel
screen = (world - center) * zoom + viewport.center
```

Two cameras exist: `target` (what the user asked for) and `current` (what is drawn).
Every frame, with `dt` clamped to 50 ms:

```
a = 1 - exp(-dt * 20)           // ~150 ms to settle, frame-rate independent
current.log_zoom = lerp(current.log_zoom, target.log_zoom, a)
current.center   = lerp(current.center,   target.center,   a)
```

Zoom interpolates in log space so a 2× step feels the same at 10% and 1000%. All input
mutates `target` only, so fast successive wheel ticks accumulate correctly and never fight
the animation. Repaint is requested while `|current − target| > ε`.

- **Zoom range:** 0.005 – 200 (clamp only to avoid float degeneracy). Level of detail (5.5)
  keeps extreme zoom-out cheap.
- **Zoom to cursor:** the world point under the cursor stays under the cursor (computed
  against `target`). Keyboard zoom anchors on the selection if any, else the viewport centre.
- **Wheel step:** `zoom *= 1.0015^(−scroll_px)` (continuous, pixel-based; a typical notch is
  ≈ 1.15×). Trackpad pinch uses egui's `zoom_delta` directly.
- **Inertia:** on drag release with velocity > 50 px/s the target keeps moving with
  `v *= exp(−dt·6)`; cancelled by any input. Off-switch in settings.
- **Key panning:** arrows / WASD / HJKL move the target at `600 px/s`, ramping ×2.5 over the
  first 400 ms of a hold; Shift multiplies by 3. Speed is in screen pixels, so it feels the
  same at any zoom.
- **Fit:** `0` fit diagram, `F` fit selection (or hovered element), `Shift+F` fit all active
  tokens, `1` = 100 %, `+`/`−`/`=` zoom ×1.25. All animated.
- **Double-click** element → zoom to it (≈ element fills 40 % of viewport); double-click
  empty → fit diagram.

### 5.2 Pointer interaction

| Input | Effect |
|---|---|
| Left drag on empty canvas | Pan |
| Middle drag anywhere, or `Space`+left drag | Pan (even over elements) |
| Left click element | Select (Ctrl+click toggles multi-select for modification) |
| Left click empty | Clear selection |
| Right click | Context menu for the element (or canvas) listing enabled commands with shortcuts |
| Hover (120 ms) | Highlight element + its flows; tooltip: name, id, type, running/incident counts, timer due, assignee |
| Wheel | Zoom (default). Setting *Wheel pans* flips: wheel pans, Ctrl+wheel zooms. Shift+wheel always pans horizontally |
| Drag on minimap | Move viewport |
| Drag reaching the view edge | The pointer is teleported to the opposite edge and the drag continues (torus). Platform capability detected from raw-window-handle: winit on X11/Windows/macOS, `wp_pointer_warp_v1` on Wayland; unavailable → ordinary drag |
| Zoom dial (next to the zoom %) | Scrub left/right to zoom continuously; one octave per 96 px; wraps the pointer too |

Cursor shapes: grab/grabbing while panning, pointer over elements, crosshair in
"pick target" mode (modification).

### 5.3 Keyboard inside the diagram

| Key | Effect |
|---|---|
| `↑↓←→` `WASD` `HJKL` | Pan (hold to accelerate, Shift faster) |
| `+ = −` | Zoom around selection/centre |
| `0` `1` `F` `Shift+F` | Fit all / 100 % / fit selection / fit tokens |
| `Tab` / `Shift+Tab` | Select next / previous element in reading order (top-left → bottom-right, lanes respected) — camera follows if the element is off-screen |
| `Alt+→` / `Alt+←` | Walk the graph: follow the outgoing / incoming sequence flow. With several candidates a numbered picker appears (`1..9`) |
| `Enter` | Primary action of selected element: open inspector (and drill into a call activity's called definition / running called instance) |
| `Esc` | Close find bar → clear search highlight → clear selection |
| `Ctrl+F` | Find element bar |
| `Ctrl+C` | Copy selected element id (Shift: name) |
| `M` | Toggle minimap, `L` layer menu, `G` grid |
| `Ctrl+Shift+C` | Copy a Cockpit URL for the current place (to paste to colleagues still on Cockpit) |

### 5.4 Find in diagram (`Ctrl+F`)

A bar docks at the top of the canvas: `[ ⌕ query ]  3 / 17  ▲ ▼  [Aa] [.*] [fields ▾]  ✕`.
Matching is live over: id, name, type name, documentation, implementation
(`class`, `delegateExpression`, `expression`, `topic`, `calledElement`, `decisionRef`,
`formKey`, `assignee`, `candidateGroups`), timer/condition expressions, message/signal/error
names, extension property values. Non-matching elements dim to 25 % opacity; matches get a
yellow outline; the current match a thicker one. `Enter` / `Shift+Enter` cycle with an
animated camera jump; results are also listed in the dock ("Matches") for clicking.
Type-filter chips (`task`, `gateway`, `event`, `flow`, `lane`) narrow quickly.

### 5.5 Rendering and level of detail

Geometry comes from BPMN DI (shapes with bounds, edges with waypoints, label bounds).
Painted with egui `Shape`s in world space transformed by the camera; everything outside the
viewport (plus margin) is culled using a flat AABB list (fine for ≤5k elements; a grid is
the fallback if profiling demands).

| Zoom | Draw |
|---|---|
| ≥ 0.6 | Full: shapes, type icons, markers, wrapped labels, flow labels, badges |
| 0.3–0.6 | Shapes, icons, labels (flow labels hidden) |
| 0.12–0.3 | Shapes and badges, no text, icons only for events |
| < 0.12 | Filled rectangles/circles per element type, lanes outlines; badges only |

Text is laid out in *world* units: line breaks are computed once per label (at a 4× reference
size, shrinking down to 60 % when the block is taller than its box) and never depend on the zoom.
Each line is then laid out at one of a few cached band sizes (quarter-octave steps) and drawn as a
glyph mesh scaled geometrically to the exact zoom, so words never re-wrap and glyphs never snap to
pixel sizes while zooming (`viewport/text.rs`).
Badges (running count, incident count) are always drawn at a constant *screen* size with a
minimum hit area of 20 px so they stay clickable at any zoom.

Element rendering spec (all vectors, no icon fonts):

- **Tasks**: 10 px rounded rect, type icon top-left (user: head+shoulders; service: gear;
  script: scroll; business rule: table; send/receive: envelope filled/outlined; manual: hand).
  Markers bottom-centre: loop ↻, parallel MI ⫼, sequential MI ≡, compensation ◂◂, ad-hoc ~.
- **Call activity**: thick (3 px) border. **Subprocess**: expanded = thin border with
  children drawn inside; collapsed = `+` box marker. Event subprocess: dotted border.
- **Events**: circle; start thin, intermediate double ring, end thick. Non-interrupting
  boundary/start: dashed. Definition icon: timer clock, message envelope, signal triangle,
  error lightning, escalation chevron, conditional lines, link arrow, compensation,
  terminate filled disc. Throwing variants filled black.
- **Gateways**: diamond; exclusive × (configurable `isMarkerVisible`), parallel +, inclusive ○,
  event-based ⬠ in double ring, complex ✱.
- **Flows**: sequence = solid polyline with filled arrowhead; default flow = slash at source;
  conditional = small diamond at source; message flow = dashed with open circle start and
  hollow arrow; association = dotted; data association = dotted with arrow.
- **Pools/lanes**: rectangles, rotated label strip at the left. **Text annotations**: open
  bracket. **Data objects/stores**, **groups**: dashed rounded rectangles.
- **Labels**: wrapped inside task bounds (centre) or at the DI label bounds for events,
  gateways and flows; shrink-to-fit before truncation with "…" and the full text on hover.

### 5.6 Overlays (layers, toggle with `L`)

| Layer | Source | Visual |
|---|---|---|
| Running tokens | `/process-instance/{id}/activity-instances` or definition `/statistics` | Blue badge top-right with count; glowing border on the active element |
| Incidents | same statistics / `/incident` | Red badge (count) top-left; red border |
| Historic path | `/history/activity-instance?processInstanceId=` | Completed nodes tinted green-grey; inferred traversed flows thicker and darker; visit count `×n` when > 1; the currently active path is distinguished from completed loops |
| Timers | `/job?processInstanceId=&timers=true` | Small clock with "in 2h 13m" at the timer event; red if overdue / failed |
| Heat (definition) | `/history/activity-instance/count` per activity (lazy, on request) | Fill opacity proportional to executions in the chosen period |
| Ids | — | Element ids drawn beneath names (for people who think in ids) |
| Grid | — | Faint dot grid in world space |

Semantic colours (identical in both themes, only the neutrals change):
running `#2F80ED`, incident `#E5484D`, completed `#30A46C`, suspended `#F5A524`,
selection accent `#7C5CFF`, search match `#F2C94C`, external task `#12A594`.

### 5.7 Minimap

Bottom-right, 200×140 px, shows all element rectangles at tiny scale, the viewport as a
rectangle, tokens as blue dots and incidents as red dots. Click/drag moves the camera
target. Toggle `M`; it hides itself automatically when the whole diagram is in view.

---

## 6. The element inspector ("what does this block actually do")

Shown in the dock whenever an element is selected or hovered-with-`Alt`. Sections, in this
fixed order, each collapsible (state remembered per section type), empty sections omitted:

1. **Identity** — type (with icon), id, name, documentation (markdown-ish plain text),
   lane / parent / pool. Ids and names copy on click.
2. **Runtime here** (context dependent) — definition: running instances at this element,
   incidents, link "Show instances here" / "Find instances that executed this";
   instance: tokens here with assignee / due date / job state / external task worker, retries.
3. **Execution** — `asyncBefore`, `asyncAfter`, `exclusive`, `jobPriority`, history TTL,
   `failedJobRetryTimeCycle`.
4. **Implementation** — service/send/business-rule/script: `class` / `delegateExpression` /
   `expression` + `resultVariable`, `topic` (external) + `taskPriority`, `decisionRef` +
   binding/version/tenant + `mapDecisionResult`, script format + resource/source;
   call activity: `calledElement` + binding/version, variable mappings (in/out, all, business key).
5. **User task** — assignee, candidate users/groups, due date, follow-up, priority, form key /
   form ref, generated form fields (id, type, label, default, validation).
6. **Events** — timer (`timeDate`/`timeDuration`/`timeCycle` with a humanised reading of ISO
   8601 durations and cron), message name, signal name, error code / error variable,
   escalation code, condition (language + expression), link name, compensation target,
   `cancelActivity` for boundary events, attached-to element (clickable).
7. **Gateway** — default flow (clickable), outgoing flows with their conditions in one table
   (this is the question people actually have: "which branch does it take?").
8. **Sequence flow** — source → target (both clickable), condition expression, default marker,
   `conditionExpression` language, listeners.
9. **Multi-instance** — parallel/sequential, collection, element variable, cardinality,
   completion condition, async settings of the MI body.
10. **I/O mapping** — input and output parameters, each with name and a value that may be a
    literal, expression, script, list or map (rendered as a nested tree).
11. **Listeners** — execution and task listeners: event, implementation (class / expression /
    delegateExpression / script), field injections.
12. **Extension properties** (`camunda:properties`) and **field injections** of the element.
13. **Raw XML** — the element's XML, pretty-printed, monospace, with "copy" (collapsed by default).

Values that look like expressions (`${…}`, `#{…}`) and ids are monospace; long values wrap;
every row supports click-to-copy and shows a tooltip with the full value.

---

## 7. Search

### 7.1 One query language, one input

Search is text first because text is the fastest keyboard interface and composes with
autocompletion; a form builder edits the *same* structure for discoverability. Grammar:

```
query   := term (ws term)*
term    := field op value | field ':' value | bare
field   := def | ver | id | bk | act | exec | state | inc | super | by | tenant
         | started | finished | var.<name> | var
op      := '=' | '!=' | '>' | '>=' | '<' | '<=' | '~'           ('~' = like, % and * wildcards)
value   := "quoted string" | 'quoted' | word | number | true | false | null
bare    := word         // uuid → id ; otherwise business key like %word%
```

| Field | Meaning | Backend mapping |
|---|---|---|
| `def:order_process` | definition key (prefix match in autocomplete) | `processDefinitionKey` |
| `ver:30` / `def:key@30` | version | `processDefinitionId` resolved via key+version |
| `id:` | process instance id | `processInstanceIds` |
| `bk:ABC*` / `bk~"ABC%"` | business key (like) | `businessKeyLike` / `processInstanceBusinessKeyLike` |
| `act:Activity_x` | currently at activity (name autocompletes to id) | `activityIdIn` / `activeActivityIdIn` |
| `exec:Activity_x` | has executed activity (history) | `executedActivityIdIn` |
| `var.amount>1000`, `var.contract="123"`, `var.flag=true`, `var:name` | variable predicates; type inferred from literal; `var:name` = variable exists | `variables[]` (`eq neq gt gteq lt lteq like`) with `variableNamesIgnoreCase`/`variableValuesIgnoreCase` toggles |
| `state:active|suspended|finished|terminated|running` | state | runtime `active/suspended`, history `finished/unfinished/state` |
| `inc:yes` / `inc:failedJob` / `inc~"timeout"` | incidents | `withIncident`, `incidentType`, `incidentMessageLike` |
| `started>2026-10-01` `finished<2026-10-20T12:00` | time bounds (date-only means local midnight) | `startedAfter/Before`, `finishedAfter/Before` |
| `super:` / `by:` / `tenant:` | super process instance, start user, tenant | obvious |

Mode is chosen automatically: if any term needs history (`exec`, `finished`, `state:finished`,
`started`) or the user toggles **Include finished**, the history API is used; otherwise
runtime. The mode is shown as a pill next to the query, clickable to force.

Autocompletion (popup under the caret): fields → values: definition keys (from the loaded
definition list), activity ids/names (for the chosen definition), variable names (collected
from result rows and remembered per definition), states, operators. `Tab` accepts.

### 7.2 Results

Virtualised table; default columns: id (short, full on hover, copy on click), business key,
definition (key v#), state, started, ended/duration, current activities (names), incidents.
Column picker adds: start user, super instance, tenant, and **variable columns**
(`+ var` → type a name → the value is fetched for the visible rows and shown; sortable
client-side). Rows: `Enter` opens, `Shift+Enter` opens in a new tab, `Space` toggles the
checkbox; `Ctrl+A` selects all loaded; the bulk toolbar appears with Suspend / Activate /
Retry jobs / Set variable / Delete / Modify → executed as Camunda **batches** when > 1,
with progress tracked in the Batches place. Searches are saved (`Ctrl+S`) and appear in the
palette by name.

### 7.3 Other search surfaces

- **Element search** in a diagram (5.4).
- **Variables filter** in the instance Variables dock: substring over names and values
  (including inside JSON) with highlighting; `/` focuses it.
- **Instance list filter** in a definition: same mini-grammar as 7.1 restricted to the
  definition; typing in it is the fastest way to "find the instance with business key X".
- **Palette** quick-jump (4.2) for ids and business keys.

---

## 8. Instance view details

- **Overview** dock tab: id, business key (editable? no — immutable in 7.x), definition link,
  version, state, start time, start user, super instance chain, tenant; counts for tokens,
  incidents, jobs, tasks; buttons = commands (Suspend/Activate, Retry all failed jobs, Delete,
  Modify, Open in Cockpit).
- **Variables**: table name / type / value preview / scope; expand → viewer (pretty JSON/XML
  tree with fold/unfold-all and in-value search, raw text, metadata: serialization format,
  object type). Edit in place with a type selector (String, Integer, Long, Double, Boolean,
  Date, Json, Xml, Object, Null); invalid JSON is flagged before sending. Add / delete /
  copy as JSON. Scope selection (process vs. a child execution) when adding.
- **Tasks**: active user tasks with assignee, candidates, created, due; claim / assign
  (user picker with recent users) / set due date; `Enter` selects the element on the diagram.
- **Incidents**: message, type, activity, created, failed job / external task link, root
  cause instance link; actions: increment retries (job or external task), set retries, view
  trace (resizable pane, searchable, copy). Annotation editing.
- **Jobs**: timers and async continuations: due date (relative + absolute), retries,
  priority, exception message; actions: execute now, set retries, set due date, suspend.
- **External tasks**: topic, worker, lock expiration, retries, error message/details;
  actions: set retries, unlock, set priority.
- **Called instances**: tree of called process instances (call activities) with their
  current activities; `Enter` drills in; ancestry breadcrumb at the top of the main area goes up.
- **Timeline**: historic activity instances as a compact Gantt (name, start, duration bar,
  assignee for tasks, cancelled marker). Click → select + fit on diagram. Zoomable time axis.
- **Modify**: token modification built on `POST /process-instance/{id}/modification`.
  Interaction: select the element → context/command "Start token before/after here",
  or select an active token → "Move token to… " → the canvas enters *pick target* mode
  (crosshair, hint in the status line, `Esc` cancels) → click target. Instructions accumulate
  in the Modify tab as a reviewable list (reorderable, removable, with per-instruction
  variables); nothing is sent until **Apply** (confirmation shows the instruction list,
  `skipCustomListeners` / `skipIoMappings` toggles). Finished instances expose **Restart**
  (`/process-definition/{id}/restart`) with the same picker.

Auto-refresh: 5 s for instance places, 10 s for lists, also while the window is unfocused (a
cockpit often sits on a second monitor; pausing then is an opt-in setting). `‖` pauses it;
the toolbar shows "updated 3 s ago".
Any write command triggers an immediate refresh of the affected place.

---

## 9. Actions, confirmations and feedback

| Class | Examples | Confirmation | Feedback |
|---|---|---|---|
| Read / navigate | open, zoom, filter | none | — |
| Safe write | claim, set retries, set due date, add/edit variable, suspend/activate | none (undo offered where the previous value is known: variable edit, assignee, due date) | toast "Variable `amount` updated · Undo" 6 s |
| Destructive | delete instance(s), cancel token, apply modification, delete variable, batch ops | inline confirm block with exact counts and object names, default focus on Cancel; `Enter` confirms only after `Tab` to Confirm or by typing `y` | persistent toast with batch link |

Toasts stack bottom-right, newest on top, max 5, hover pauses their timer; errors stay until
dismissed and carry "Copy details" (HTTP status, Camunda `type` + `message`). The status line
shows the number of requests in flight; the engine pill shows connectivity and latency.

---

## 10. Visual system

- **Themes**: light and dark, following the OS by default, switchable (`Ctrl+Shift+T`). The
  diagram canvas is `bg0`, panels `bg1`, cards/rows `bg2`, borders `bg3`; text `fg0`
  (primary), `fg1` (secondary), `fg2` (disabled). Semantic colours in 5.6 are shared.
- **Type scale**: 11 (meta), 13 (body, default), 15 (section titles), 20 (place title);
  mono 12 for ids, expressions, JSON. Line height 1.4.
- **Spacing**: 4 px inner, 8 px between controls, 12 px section padding, 16 px between
  sections. Corner radius 4 (controls) / 6 (panels). Focus ring 2 px accent.
- **Density** setting scales the type scale and spacing by 0.9 / 1.0 / 1.15.
- **Tables**: 28 px rows, zebra at 3 % contrast, sticky header, sortable columns with an
  arrow, resizable, column picker in the header context menu; keyboard row cursor shown as a
  left accent bar.
- **Badges**: 16 px pill, bold 11 px number, semantic fill, white text.
- **Chips** (search terms, layer toggles): 22 px, rounded, `×` on hover and `Backspace` to
  remove when the caret is adjacent.
- **Motion**: 120–150 ms exponential for panels, highlights and the camera; none for tables.

---

## 11. Architecture

```
crates/
  bpmn/        BPMN 2.0 XML → Model { elements, flows, lanes, DI shapes/edges, extension data }.
               Pure, no I/O. Reading order, graph adjacency, bounding boxes, label layout hints.
  camunda/     Typed async REST client for Camunda 7 (engine-rest): definitions, instances,
               history, variables, incidents, jobs, external tasks, tasks, modification,
               batches, statistics. Pagination helpers. Error type carrying Camunda's
               {type, message}. No UI types.
  cockpit/     The egui application (binary `flightdeck`).
    app.rs        eframe::App: frame loop, message drain, layout of the shell
    state/        AppState, Tab, Place, per-place view state, Resource<T>
    net.rs        background tokio runtime, Request → Response channel, in-flight registry,
                  polling scheduler
    commands/     Command registry, shortcuts, palette matching, context menus
    viewport/     Camera, smoothing, input mapping, culling, LOD, renderer, overlays, minimap,
                  hit testing, find bar
    inspector/    Element inspector sections
    places/       definitions.rs, definition.rs, instance.rs, search.rs, incidents.rs,
                  batches.rs, settings.rs
    widgets/      table, kv rows, badges, chips, toasts, json tree, confirm block
    search/       query grammar (parser + printer), autocompletion, backend mapping
    theme.rs      tokens, fonts, light/dark
```

Data flow is unidirectional:

```
input → Command::execute(&mut State, &Effects) → State mutated + Effects queued
Effects (HTTP) run on the tokio thread → Msg sent through a channel → drained at frame start
→ State.resources updated → UI draws from State
```

`Resource<T> = { status: Idle | Loading | Ready | Error, data: Option<T>, fetched_at,
error: Option<ApiError> }` keeps the last good value during refreshes and errors. Every
`Resource` has a `key` (the request signature) so a place that re-mounts re-uses the cache
instead of flashing empty.

Threading: UI thread (egui) + one tokio multi-thread runtime for HTTP (max 8 concurrent
requests to the same engine, FIFO, with cancellation of superseded polls). The UI never
awaits.

Persistence (TOML in the OS config dir): engine profiles (passwords in the OS keyring when
available, otherwise plain with a warning), window geometry, theme, density, input
settings, saved searches, dock widths, collapsed sections, column layouts.

---

## 12. Performance budgets

| Item | Budget |
|---|---|
| Parse a 1.1 MB BPMN file | < 80 ms |
| Frame with 2 000 elements fully visible at full LOD | < 6 ms on integrated graphics |
| Frame while panning (cached galleys) | < 3 ms |
| Definition list for 200 definitions with counts | 2 requests (`/process-definition?latestVersion=true`, `/process-definition/statistics?incidents=true`) |
| Instance view first paint | 1 request (instance) then parallel: diagram XML (cached per definition id), activity instances, variables, incidents, jobs |
| Idle CPU | 0 — repaint only on input, animation, or data arrival |

---

## 13. Testing strategy

- `bpmn`: unit tests on a synthetic fixture covering every element type; an ignored test
  parses every `*.bpmn` under `$FLIGHTDECK_BPMN_DIR` and asserts no element lacks DI, which
  we run against the user's real diagrams.
- `camunda`: serde round-trip tests on captured JSON; integration tests gated by
  `$CAMUNDA_URL` run against a local Camunda Run.
- `cockpit/viewport`: pure unit tests for camera maths (zoom-to-cursor invariance, log-space
  interpolation convergence, fit, culling) and the search grammar (parse ↔ print, mapping to
  request bodies).
- UI: `egui_kittest` headless runs for the shell, the palette and the inspector, plus a
  `--screenshot <path>` flag on the binary that loads a diagram, renders N frames and writes a
  PNG, used to eyeball rendering during development.

---

## 14. Milestones

1. **Skeleton + diagram** — workspace, BPMN parser, viewport with camera/LOD/minimap/find,
   definitions list, definition place with statistics badges, inspector. *(the user's
   diagrams must render and feel smooth here)*
2. **Instances** — instance place with tokens, historic path, variables viewer/editor, tasks,
   incidents (+trace), jobs, external tasks, called instances, timeline; search language +
   results table; tabs/history; palette.
3. **Actions** — suspend/activate, delete, retries, variable writes, assignee, due dates,
   modification & restart, batches; confirmations and undo.
4. **Polish** — settings, persistence, themes, density, screenshots in README, wasm build.

---

## 15. AI agents: the built-in MCP server

Flightdeck embeds a Model Context Protocol server (Streamable HTTP, JSON-RPC 2.0, localhost
only, optional bearer token) so an agent can do everything a human operator can, through the
same code paths. It is off by default and toggled from the top-bar pill ("MCP :7788"), from
Settings → MCP server, or with `flightdeck --mcp`.

Design rules:

- **Tools run on the UI thread.** The HTTP thread queues a call and blocks; the frame loop
  drains the queue, executes the tool against `&mut App` and answers (engine calls are spawned
  on the I/O runtime and answer when done). There is no second code path: `ui_select` is the
  same `DiagramView::select` a click uses, `ui_filter` writes the same filter string the user
  types into.
- **Not-ready is not an error.** A tool that needs a diagram or the definition list that is
  still downloading is re-queued for a later frame (up to ~15 s) instead of failing.
- **Two families.** Engine tools (`list_definitions`, `get_element`, `search_instances`,
  `get_instance`, `set_variable`, `modify_instance`, `batch`, …) answer structured JSON;
  UI tools (`ui_state`, `ui_navigate`, `ui_select`, `ui_fit`, `ui_find`, `ui_dock`,
  `ui_filter`, `ui_key`, `ui_type`, `ui_command`, `ui_screenshot`, `ui_confirm`, `ui_view_xml`,
  `ui_xml`, `ui_compare`, `ui_diff`, …) read and drive the interface; `diff_bpmn` answers a
  semantic diff of two documents without any UI. `ui_screenshot` returns a PNG as MCP image content so the agent can see
  what the user sees.
- **Destructive tools require `confirm: true`** (delete, modification, restart, batch,
  complete task, resolve incident, suspend definition), mirroring the human confirmation
  dialog; `ui_confirm` answers the human dialog when the agent triggered it through the UI.
- **Synthetic input** is limited to what egui can accept mid-frame: key presses and typed text
  (`ui_key`, `ui_type`). Pointer clicks are not synthesised; every clickable thing has a
  semantic tool instead (`ui_select`, `ui_dock`, `ui_tabs`, `ui_command`, `ui_hit_test`).
- **Clients:** Claude Code reads the project `.mcp.json` (`type: http`); any stdio-only client
  can use `flightdeck --mcp-stdio http://127.0.0.1:7788/mcp` as a bridge. A `flightdeck` skill in
  `.claude/skills` teaches the workflow (orient → read → act → show).

## 16. Decisions log

- *Wheel zooms by default.* Cockpit's "wheel scrolls" is the single most complained-about
  behaviour; people navigating a diagram want to zoom. Panning has drag, keys and Shift+wheel.
- *No bounce or easing curves beyond exponential.* Exponential smoothing is frame-rate
  independent, interruptible and never overshoots.
- *Text query language rather than a chip builder.* Keyboard speed and copy/paste-ability of
  queries (into chat with a colleague) win; the form exists for discovery and always
  round-trips to the same text.
- *Element inspector shows raw attributes, not a curated subset.* The whole point of the
  pain is "I can't see what this does"; curation is done by ordering and collapsing, not by
  omission.
- *Window placement for development.* `FLIGHTDECK_NO_FOCUS=1` creates the window without
  taking focus: `with_active(false)` on Windows/macOS, and on X11 (where winit ignores that)
  `_NET_WM_USER_TIME=0` set before the window is mapped. Wayland cannot decline focus, so a
  no-focus start uses Xwayland when it exists. `FLIGHTDECK_ON_TOP=1` keeps the unraised window
  visible (a covered window paints nothing, so screenshots would stall) and
  `FLIGHTDECK_WINDOW_POS=x,y` positions it. Test runs never steal the user's focus.
- *Screenshots never wait silently.* The image is read back from the GPU on a later frame, so
  while one is pending the app keeps requesting repaints (an idle unfocused window paints
  none otherwise), and after a timeout the caller gets the reason (window covered, minimized).
- *Native first, wasm later.* Native gets the smoothest input and no CORS games; the code
  stays wasm-compatible (no blocking I/O on the UI thread, `reqwest` with the wasm feature).

## 17. XML view and diffs

- **Every diagram knows its XML.** Right-click an element, a flow or the background: *View XML*
  (`Ctrl+U`), *Open XML with…* (the system application chooser: the XDG OpenURI portal with
  `ask`, `SHOpenWithDialog` on Windows; absent elsewhere), *Open XML in editor* (only when a
  command is set in Settings, with `{file}` `{line}` `{col}` placeholders, e.g.
  `code -g {file}:{line}:{col}`) and *Copy path:line:col*. Columns count characters, as editors
  do. An engine definition has no file, so it is written once to
  `<data dir>/definitions/<key>-v<version>-<id8>.bpmn` (no colons, so `path:line:col` stays
  unambiguous).
- **One XML tab per document.** *View XML* switches to the tab already showing that document
  (`NavMode::Reuse`) and points it at the new element instead of opening another. The highlight
  covers the element's exact byte range (start tag to end tag), is a chip in the toolbar and
  goes away with ✕ or Esc. Right-click a line: show that element in the diagram (the innermost
  element containing the line, or the `bpmnElement` of a DI shape), highlight it, copy the line
  or its location.
- **The viewer is virtualised** (only visible rows are laid out) and its highlighter is
  line-oriented with the state carried across lines, so multi-line attribute values, comments
  and CDATA are coloured correctly from any starting row. Very long lines are cut at 6000
  characters for drawing.
- **Diffs are semantic first.** `bpmn::diff` matches elements and flows by id, then by
  (kind, unique name) for regenerated ids, then flows by their matched endpoints. Each item
  gets a status — added, removed, changed (with every changed property: attributes,
  documentation, event definitions, conditions, scripts, I/O parameters, listeners, fields,
  properties, lane, parent…) or moved (layout only) — and process/file-level changes are listed
  separately. The unified view draws the new diagram with the removed items merged in as dashed
  red ghosts; added is green, changed amber with `~N` (number of properties), moved blue with a
  dashed outline where it used to be; unchanged items are dimmed (toggle). Before / After / Side
  by side (cameras and selection locked together) use the same marks. `N` / `Shift+N` (or F7)
  step through the changes in list order.
- **XML diffs are git diffs.** Patience line diff, two line-number columns and a `+`/`−` gutter,
  word emphasis inside a changed line pair (none when most of the line changed — the line colour
  already says it), unchanged runs folded to three lines of context (click to unfold; the element
  being shown is always unfolded), *Whole file* to show everything.
- **Documentation is Markdown-lite and last.** BPMN `documentation` (and a process definition's
  description, which Camunda takes from it with line breaks intact) renders with kept line
  breaks, headings, lists, quotes, code, emphasis and links; plain text renders as itself
  (`SOME_ROLE_CODE` keeps its underscores). It is always the last field of a viewer and folds
  when long, so it never pushes the short fields out of view.

## Localisation

UI text comes from Fluent language packs (`locales/<tag>.ftl`); nothing is compiled in. The
binary resolves the search path (`--locale-dir` / `FLIGHTDECK_LOCALE_DIR`, else user config and
data dirs, `$XDG_DATA_DIRS/flightdeck/locales`, and next to the executable) and hands it to the
app, which discovers packs, picks one (`Settings → Language`: system or explicit; system matches
the OS locale by full tag, then by language, then English) and activates it. Lookups fall back to
English, then to the message id. Labels that come from the model crate as English constants
(element kinds, key/value row labels, section titles, dock tabs, table columns) are mapped through
slug-based ids (`bpmn-user-task`, `kv-business-key`, `section-identity`, `dock-inspector`,
`col-started`) so adding a pack never requires code changes. Tests enforce that every id used in
code exists in `en-US.ftl` and that all shipped packs define the same ids. MCP tool output stays
English.
