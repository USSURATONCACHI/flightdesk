# Flightdeck — Deutsch
language-name = Deutsch

## Common words
common-remove = Entfernen
common-reset = zurücksetzen
common-on = Ein
common-off = Aus
common-enabled = Aktiviert
common-none = keiner
common-version = Version
common-cancel = Abbrechen
common-close = Schließen
common-copy = Kopieren
common-copied = kopiert
common-details = Details
common-yes = ja
common-no = nein
common-ok = OK
common-loading = wird geladen…
common-error = Fehler
common-retry = Wiederholen
common-search = Suchen
common-open = Öffnen
common-open-new-tab = In neuem Tab öffnen
common-refresh = Aktualisieren
common-all = alle
common-unknown = unbekannt

## Themes & density
theme-system = System
theme-light = Hell
theme-dark = Dunkel
density-compact = Kompakt
density-normal = Normal
density-comfortable = Komfortabel

## Connection fields
connect-name = Name
connect-rest-url = REST-URL
connect-user = Benutzer
connect-password = Passwort
connect-remember = merken
connect-cockpit-url = Cockpit-URL
connect-cockpit-url-hint = Wird von „Cockpit-Link kopieren“ verwendet. Leer lassen, um sie aus der REST-URL abzuleiten.
connect-allow-self-signed = Selbstsigniertes TLS erlauben
connect-connect = Verbinden
connect-connected = verbunden
connect-add = + Verbindung hinzufügen
connect-default-name = Engine { $n }

## Settings place
settings-title = Einstellungen
settings-saved-automatically = werden automatisch gespeichert
settings-connections = Verbindungen
settings-auto-connect = Beim Start mit der letzten Engine verbinden
settings-appearance = Darstellung
settings-theme = Farbschema
settings-language = Sprache
settings-language-system = System
settings-language-none = keine Sprachpakete gefunden; es werden Nachrichten-IDs angezeigt
settings-language-warnings = { $n ->
    [one] { $n } Problem im Sprachpaket (Details beim Überfahren)
   *[other] { $n } Probleme im Sprachpaket (Details beim Überfahren)
}
settings-density = Dichte
settings-diagram-input = Diagrammbedienung
settings-mouse-wheel = Mausrad
settings-wheel-zooms = zoomt (Strg+Rad verschiebt)
settings-wheel-pans = verschiebt (Strg+Rad zoomt)
settings-inertia = Nachlaufen nach dem Anstoßen des Diagramms
settings-zoom-speed = Zoomgeschwindigkeit
settings-reset-to = zurück auf { $value }×
settings-zoom-global = Global
settings-zoom-global-tip = Skaliert jede Zoomgeste: Rad, Pinch, Zoomrad und Tastatur
settings-zoom-mouse = Maus
settings-zoom-mouse-tip = Zusätzlicher Faktor für Zeigergesten (Rad, Pinch, Zoomrad), zusätzlich zu Global
settings-zoom-keyboard = Tastatur
settings-zoom-keyboard-tip = Zusätzlicher Faktor für die Tasten + / −, zusätzlich zu Global
settings-zoom-alt = Alt gedrückt
settings-zoom-alt-tip = Verschiebe- und Zoomgeschwindigkeit bei gedrückter Alt-Taste, zusätzlich zu allem anderen. Unter 1 verlangsamt für Feinarbeit
settings-zoom-effective = effektiv: Maus { $mouse }×, Tastatur { $keyboard }×, mit Alt zusätzlich ×{ $alt }
settings-unlimited-drag = Unbegrenztes Ziehen
settings-unlimited-drag-on-tip = Zeiger an den gegenüberliegenden Rand versetzen, wenn das Ziehen den Fensterrand erreicht
settings-unlimited-drag-off-tip = Ziehen endet am Rand
settings-find = Im Diagramm suchen (Strg+F)
settings-find-instances = Auch laufende Instanzen suchen und die Aktivitäten hervorheben, an denen sie warten
settings-find-business-keys = nach Business-Key (enthält)
settings-find-ids = nach Prozessinstanz-ID (enthält)
settings-refresh = Aktualisierung
settings-refresh-instances = Instanzansichten (Sekunden)
settings-refresh-lists = Listen (Sekunden)
settings-page-size = Seitengröße
settings-pause-unfocused = Automatische Aktualisierung pausieren, solange das Fenster nicht fokussiert ist
settings-confirm-destructive = Destruktive Aktionen bestätigen
settings-mcp = MCP-Server (KI-Agenten)
settings-mcp-description = Stellt Flightdeck KI-Agenten über das Model Context Protocol bereit (Streamable HTTP auf localhost). Die Werkzeuge decken die Engine (Definitionen, Instanzen, Variablen, Incidents, Jobs, Aufgaben, Modifikation, Batches) und die Oberfläche selbst ab (Navigation, Auswahl, Zoom, Dock, Suche, Screenshots, Tasten).
settings-mcp-port = Port
settings-mcp-token = Bearer-Token (optional)
settings-mcp-listening = lauscht auf { $url }
settings-mcp-copy-url = URL kopieren
settings-mcp-copy-config = Claude-Code-Konfiguration kopieren
settings-mcp-stats = { $requests } Anfragen · { $calls } Werkzeugaufrufe · { $errors } Fehler
settings-mcp-last = zuletzt: { $tool }
settings-saved-searches = Gespeicherte Suchen
settings-saved-searches-empty = Speichern Sie eine Abfrage auf der Suchseite (☆).
settings-about = Über
settings-about-text = Flightdeck — ein schnelles, tastaturorientiertes Cockpit für Camunda 7.
settings-file = Einstellungsdatei
settings-not-persisted = Einstellungen werden in dieser Sitzung nicht gespeichert

## Connection screen
connect-title = Verbindungen
connect-list-hint = Klick · Strg fügt hinzu · Umschalt wählt Bereich · Alt entfernt · in den Papierkorb ziehen
connect-unnamed = (ohne Namen)
connect-add-tip = Verbindung hinzufügen
connect-new-name = Verbindung { $n }
connect-empty = Noch keine gespeicherten Verbindungen.
connect-bin-release = Loslassen zum Löschen
connect-bin-hint = Verbindungen zum Löschen hierher ziehen
connect-delete-one = Verbindung „{ $name }“ löschen?
connect-delete-many = { $n } Verbindungen löschen?
    • { $list }
connect-delete-title = Verbindungen löschen
connect-deleted = { $n ->
    [one] { $n } Verbindung gelöscht
   *[other] { $n } Verbindungen gelöscht
}
connect-subtitle = Mit einer Camunda-7-Engine verbinden
connect-select-hint = Wählen Sie links eine Verbindung oder legen Sie mit „+“ eine an.
connect-user-hint = optional (Basic Auth)
connect-show-password = Passwort anzeigen
connect-remember-password = Passwort merken
connect-remember-tip = In der Einstellungsdatei gespeichert (nur für den Besitzer lesbar)
connect-connecting = Verbinde…
connect-url-note = Die URL wird normalisiert: `host:8080`, `…/camunda/app/cockpit/…` und `…/engine-rest` funktionieren alle.
connect-open-bpmn = .bpmn-Datei öffnen
connect-open-bpmn-tip = Offline-Ansicht: keine Engine nötig (Strg+O; Dateien lassen sich auch ins Fenster ziehen)
common-delete = Löschen
common-suspend = Anhalten
common-activate = Aktivieren

## Toasts and errors
toast-connected = Verbunden mit Camunda { $version }
error-auth = Authentifizierung fehlgeschlagen: Benutzer und Passwort prüfen.
error-not-camunda = 404 unter /version: die URL zeigt nicht auf eine Camunda-REST-API.
toast-mcp-listening = MCP-Server lauscht auf { $url }
toast-mcp-failed = MCP-Server konnte auf Port { $port } nicht starten
toast-mcp-stopped = MCP-Server gestoppt
toast-no-definition-key = Keine Definition mit Schlüssel { $key }
toast-copied = Kopiert: { $what }
toast-job-retries = Job-Wiederholungen auf { $n } gesetzt
toast-job-retries-failed = Job-Wiederholungen konnten nicht gesetzt werden
toast-external-retries = Wiederholungen der externen Aufgabe auf { $n } gesetzt
toast-external-retries-failed = Wiederholungen der externen Aufgabe konnten nicht gesetzt werden
definition-suspend-title = Definition anhalten
definition-suspend-body = Prozessdefinition { $id } einschließlich aller laufenden Instanzen anhalten?
definition-activate-title = Definition aktivieren
definition-activate-body = Prozessdefinition { $id } einschließlich aller laufenden Instanzen aktivieren?
toast-definition-suspended = Definition angehalten
toast-definition-activated = Definition aktiviert
toast-suspension-failed = Anhaltestatus konnte nicht geändert werden
perms-warning = { $n ->
    [one] Die Einstellungsdatei ist für andere Benutzer lesbar ({ $n } Pfad). Gespeicherte Passwörter sollten privat sein.
   *[other] Einstellungsdateien sind für andere Benutzer lesbar ({ $n } Pfade). Gespeicherte Passwörter sollten privat sein.
}
perms-fix = Rechte korrigieren
perms-fixed = Einstellungsverzeichnis ist jetzt privat (700 / 600)
perms-fix-failed = Einige Rechte konnten nicht geändert werden

## Top bar, status bar, tabs
status-connecting = verbinde…
status-offline = offline
topbar-refresh-everything = Alles aktualisieren
topbar-disconnect = Trennen
tab-close-tip = Schließen (Strg+W)
tab-new-tip = Neuer Tab (Strg+T)
topbar-shortcuts-tip = Tastenkürzel (?)
topbar-mcp-off = MCP aus
topbar-mcp-tip = Eingebauter MCP-Server für KI-Agenten. Zum Umschalten klicken.
topbar-settings-tip = Einstellungen (Strg+,)
topbar-theme-tip = Farbschema: System / hell / dunkel (Strg+Umschalt+T wechselt)
topbar-search = Suchen oder springen…
nav-forward-tip = Vorwärts (Alt+→)
nav-back-tip = Zurück (Alt+←)
refresh-resume-tip = Automatische Aktualisierung fortsetzen (Strg+Umschalt+P)
refresh-pause-tip = Automatische Aktualisierung pausieren (Strg+Umschalt+P)
refresh-now-tip = Jetzt aktualisieren (R / F5)
refresh-updating = wird aktualisiert…
refresh-updated = aktualisiert { $ago }
status-requests = { $n ->
    [one] { $n } Anfrage
   *[other] { $n } Anfragen
}
status-paused = automatische Aktualisierung pausiert
status-paused-unfocused = Aktualisierung pausiert, Fenster nicht fokussiert

## Diagram context menu
ctx-inspect = Untersuchen
ctx-copy-id = ID kopieren
ctx-copy-name = Namen kopieren
ctx-instances-waiting = Hier wartende Instanzen
ctx-instances-executed = Instanzen, die dies ausgeführt haben…
ctx-open-called = Aufgerufenen Prozess { $key } öffnen
ctx-go-source = Zur Quelle: { $name }
ctx-go-target = Zum Ziel: { $name }
ctx-diagram = Diagramm
ctx-fit-diagram = Diagramm einpassen

## Commands (palette, help)
cat-diagram = Diagramm
cat-general = Allgemein
cat-navigate = Navigation
cat-tabs = Tabs
cat-view = Ansicht
cmd-back = Zurück
cmd-close-tab = Tab schließen
cmd-copy-cockpit-link = Link für Camunda Cockpit kopieren
cmd-copy-place-id = Aktuelle ID kopieren (Definition / Instanz)
cmd-disconnect = Verbindung zur Engine trennen
cmd-find-in-diagram = Element im Diagramm suchen
cmd-fit-diagram = Diagramm einpassen
cmd-focus-filter = Filter- / Suchfeld fokussieren
cmd-forward = Vorwärts
cmd-go-batches = Zu den Batches
cmd-go-definitions = Zu den Prozessdefinitionen
cmd-go-incidents = Zu den Incidents
cmd-help = Tastenkürzel
cmd-new-tab = Neuer Tab
cmd-next-tab = Nächster Tab
cmd-open-settings = Einstellungen öffnen
cmd-palette = Befehlspalette
cmd-prev-tab = Vorheriger Tab
cmd-refresh = Jetzt aktualisieren
cmd-search-instances = Prozessinstanzen suchen
cmd-toggle-dock = Seitenleiste ein-/ausblenden
cmd-toggle-pause = Automatische Aktualisierung pausieren / fortsetzen
cmd-toggle-theme = Farbschema wechseln: System → hell → dunkel
cmd-zoom-in = Vergrößern
cmd-zoom-out = Verkleinern

## Palette
palette-open-instance = Instanz { $id } öffnen
palette-kind-instance-id = Prozessinstanz-ID
palette-open-as-definition = Als Definitions-ID öffnen
palette-open-definition = Definition { $id } öffnen
palette-kind-definition-id = Prozessdefinitions-ID
palette-search-business-key = Instanzen mit Business-Key „{ $q }“ suchen
palette-search-instances = Instanzen suchen: { $q }
common-no-matches = Keine Treffer
toast-refresh-paused = Automatische Aktualisierung pausiert
toast-refresh-resumed = Automatische Aktualisierung fortgesetzt

## Help overlay
help-title = Tastatur & Maus
help-diagram = Diagramm
help-everywhere = Überall
help-close-hint = ? oder Esc zum Schließen
help-g-dock-tab = Dock-Tab
help-g-tables = Tabellen: bewegen · öffnen · in neuem Tab öffnen
help-g-middle-click = in neuem Tab öffnen
help-g-slash = Filterfeld fokussieren
help-g-alt = Element unter dem Zeiger untersuchen
help-d-wheel = Zoom am Zeiger
help-d-alt = schneller verschieben und zoomen (×2 standardmäßig, siehe Einstellungen)
help-d-shift-wheel = horizontal verschieben
help-d-drag = verschieben (anstoßen für Nachlauf)
help-d-arrows = verschieben (gedrückt halten beschleunigt, Umschalt schneller)
help-d-plus-minus = vergrößern / verkleinern (Alt: um den Alt-Faktor)
help-d-fit = alles einpassen · 100 % · Auswahl einpassen · Tokens einpassen
help-d-double-click = auf Element zoomen (leer: alles einpassen)
help-d-tab = nächstes / vorheriges Element entlang des Flusses
help-d-alt-arrows = ausgehendem / eingehendem Fluss folgen (1-9 wählt)
help-d-enter = untersuchen · in Call Activity springen
help-d-find = Element oder Instanz suchen (Enter/Umschalt+Enter blättern)
help-d-copy = Element-ID kopieren
help-d-toggles = Minikarte · Raster · Element-IDs
help-d-right-click = Kontextmenü
help-d-esc = Suche schließen → Auswahl aufheben

## Diagram: find bar, zoom, tooltips
pick-banner = { $label } — Zielelement anklicken, Esc bricht ab
find-hint = Element, Business-Key oder Instanz-ID suchen…
find-no-matches = keine Treffer
find-prev-tip = Vorheriger Treffer (Umschalt+Enter)
find-next-tip = Nächster Treffer (Enter)
find-close-tip = Schließen (Esc)
find-case-tip = Groß-/Kleinschreibung beachten
find-flows-tip = Sequenzflüsse einbeziehen
find-cat-task = Task
find-cat-gateway = Gw
find-cat-event = Ev
find-cat-call = Call
find-cat-subprocess = Sub
find-only-task = Nur Tasks
find-only-gateway = Nur Gateways
find-only-event = Nur Ereignisse
find-only-call = Nur Call Activities
find-only-subprocess = Nur Unterprozesse
find-count-elements = { $n ->
    [one] { $n } Element
   *[other] { $n } Elemente
}
find-searching-instances = suche Instanzen…
find-count-instances = { $n ->
    [one] { $n } Instanz
   *[other] { $n } Instanzen
} an { $a ->
    [one] { $a } Aktivität
   *[other] { $a } Aktivitäten
}
find-legend-instance-short = Instanztreffer
find-go-to = Gehe zu { $name }
zoom-tip = Zoom. Klick für Voreinstellungen. Rad zoomt, Umschalt+Rad verschiebt, 0 passt ein, 1 = 100 %
zoom-fit-diagram = Diagramm einpassen
zoom-fit-selection = Auswahl einpassen
zoom-minimap = Minikarte
zoom-grid = Raster
zoom-ids = Element-IDs
zoom-dial-tip = Nach links oder rechts ziehen, um beliebig weit zu zoomen
tip-running = { $n } laufend
tip-incidents = { $n ->
    [one] { $n } Incident
   *[other] { $n } Incidents
}
tip-executed = ausgeführt ×{ $n }
tip-timer = Timer { $t }
tip-default-flow = Standardfluss
find-explain-element = Elementtreffer: { $field } „{ $snippet }“
find-explain-instances = { $n ->
    [one] { $n } laufende Instanz hier trifft zu:
   *[other] { $n } laufende Instanzen hier treffen zu:
}
find-explain-business-key = Business-Key { $key }
find-explain-id = ID { $id }
find-explain-join = Ein Token dieser Instanz wartet an diesem Join auf die übrigen Zweige.
find-explain-event-gateway = Die Instanz wartet hier auf eines der folgenden Ereignisse.
find-explain-more = … und { $n } weitere
find-explain-striped = gestreift: Element und Instanz treffen zu
find-explain-current = Aktueller Treffer { $i } von { $n } (Enter / Umschalt+Enter blättern)
find-legend-element = Elementtext trifft zu ({ $n })
find-legend-instance = eine laufende Instanz hier trifft zu ({ $n })
find-legend-both = beides ({ $n })
find-legend-current = aktueller Treffer · Enter blättert

## Places, lists and hints
place-definitions = Definitionen
place-incidents = Incidents
place-batches = Batches
definitions-title = Prozessdefinitionen
definitions-load-failed = Definitionen können nicht geladen werden: { $error }
definitions-empty = Keine Prozessdefinitionen deployt.
definitions-copy-key = Schlüssel kopieren
definitions-copy-id = Definitions-ID kopieren
common-all-versions = Alle Versionen
common-nothing-matches-filter = Nichts passt zum Filter.
common-search-instances = Instanzen suchen
common-copy-business-key = Business-Key kopieren
common-open-definition = Definition öffnen
common-select-on-diagram = Im Diagramm auswählen
common-click-to-copy = Zum Kopieren klicken
common-find = Suchen
common-save = Speichern
common-set = Setzen
common-edit = Bearbeiten
common-clear = Leeren
common-apply = Anwenden
common-restart = Neu starten
common-resume = Fortsetzen
common-prev = zurück
common-next = weiter
common-now = jetzt
common-undo = Rückgängig
state-suspended = angehalten
state-running-lower = laufend
n-running = { $n } laufend
n-incidents = { $n ->
    [one] { $n } Incident
   *[other] { $n } Incidents
}
n-tokens = { $n ->
    [one] { $n } Token
   *[other] { $n } Tokens
}
n-open = { $n } offen
n-days = { $n ->
    [one] { $n } Tag
   *[other] { $n } Tage
}
hint-list = ↑↓ bewegen · Enter öffnen · Umschalt+Enter neuer Tab · / Filter
hint-definition = Rad zoomt · Ziehen verschiebt · Tab nächstes Element · Enter untersuchen / in Call Activity springen · Klick auf ein Abzeichen filtert Instanzen · Strg+F suchen · ? Hilfe
hint-instance = Umschalt+F Tokens einpassen · Enter untersuchen / in aufgerufene Instanz springen · Rechtsklick: Tokens modifizieren · Strg+1…9 Dock-Tabs · ? Hilfe
hint-pick = Zielelement im Diagramm wählen · Esc bricht ab
hint-incidents = ↑↓ bewegen · Enter Instanz öffnen · Umschalt+Enter neuer Tab · / Filter
hint-search = ↑↓ bewegen · Enter öffnen · Umschalt+Enter neuer Tab · Leertaste auswählen · Strg+A alle · Doppelklick öffnen · / Abfrage
hint-file = Ziehen/Rad: verschieben & zoomen · 0 einpassen · F Auswahl einpassen · Tab nächstes Element · Strg+F suchen · ? Hilfe
definition-show-instances = Instanzen anzeigen
definition-find-element = Element suchen
definition-search-instances-tip = Instanzsuche für diese Definition öffnen
diagram-load-failed = Diagramm kann nicht geladen werden: { $error }
diagram-loading = Diagramm wird geladen…
inspector-empty-definition = Wählen Sie ein Element, um es zu untersuchen.
    Tab wechselt Elemente · Strg+F sucht · Alt gedrückt halten untersucht beim Überfahren.
inspector-empty-instance = Wählen Sie ein Element, um es zu untersuchen.
    Tab wechselt Elemente · Alt gedrückt halten untersucht beim Überfahren.
inspector-empty-file = Wählen Sie ein Element, um es zu untersuchen.
    Tab wechselt Elemente, Strg+F sucht sie.
toast-element-id-copied = Element-ID kopiert
instances-at = an
instances-clear-activity-filter = Aktivitätsfilter aufheben
instances-empty = Keine passenden laufenden Instanzen.
definition-no-incidents = Keine offenen Incidents in dieser Version.
instance-short = Instanz { $id }
incident-retry-job-tip = Job-Wiederholungen auf 1 setzen
incident-retry-external-tip = Wiederholungen der externen Aufgabe auf 1 setzen
incident-retry-tip = Wiederholungen auf 1 setzen
incidents-group-by-message = Nach Meldung gruppieren
incidents-filter-hint = Nach Meldung, Typ, Definition, Aktivität filtern
incidents-empty = Keine offenen Incidents. 🎉
incidents-none = Keine Incidents.
batches-running = Laufend
batches-finished = Abgeschlossen (Historie)
batches-empty-running = Keine laufenden Batches.
batches-empty-finished = Keine abgeschlossenen Batches.
batch-delete-title = Batch löschen
batch-delete-body = Diesen Batch und seine verbleibenden Jobs löschen? Bereits ausgeführte Jobs werden nicht rückgängig gemacht.
toast-batch-deleted = Batch gelöscht
toast-batch-delete-failed = Batch konnte nicht gelöscht werden
toast-batch-updated = Batch aktualisiert
toast-batch-update-failed = Batch konnte nicht aktualisiert werden
started-when = gestartet { $when }
created-when = erstellt { $when }
created-by = erstellt von { $user }
file-diagram-warnings = { $n ->
    [one] { $n } Diagrammwarnung (siehe Tab Inspektor)
   *[other] { $n } Diagrammwarnungen (siehe Tab Inspektor)
}
file-open-failed = { $path } kann nicht geöffnet werden: { $error }
inspector-running-here = { $n } hier laufend
inspector-running-here-tip = Instanzen anzeigen, die an diesem Element warten
inspector-executed-tip = Historiensuche: ausgeführte Aktivität
mi-sequential = sequenziell
mi-parallel = parallel
io-inputs = Eingaben
io-outputs = Ausgaben
inspector-copy-xml = XML kopieren
confirm-hint = Tab zum Bestätigen, oder y drücken
toast-hide-details = Details ausblenden

## Instance place
instance-tab-title = Inst. { $id }
job-failed = fehlgeschlagen
instance-super-tip = Übergeordnete Instanz { $id }
instance-copy-bk-tip = Klicken, um den Business-Key zu kopieren
toast-instance-id-copied = Instanz-ID kopiert
instance-tokens-tip = Umschalt+F passt die Kamera auf die Tokens ein
instance-not-found = Instanz nicht gefunden (gelöscht oder Historie entfernt)
find-element-tip = Element suchen (Strg+F)
instance-loading = Instanz wird geladen…
trace-filter-hint = Zeilen filtern
extra-task = Aufgabe: { $who }
assigned-to = zugewiesen an { $user }
task-unassigned = nicht zugewiesen
due-when = fällig { $when }
extra-job = Job: fällig { $due } · Wiederholungen { $retries }
extra-external = extern: Topic { $topic } · { $lock }
locked-by = gesperrt von { $worker }
ext-unlocked = nicht gesperrt
ctx-modify = Modifizieren
mod-start-before = Token davor starten
mod-start-after = Token danach starten
mod-cancel-here = Tokens hier abbrechen
mod-move-from = Token von hier verschieben nach…
token = Token
token-async = asynchrone Fortsetzung
incident-of-type = Incident ({ $type })
instance-restart = Neu starten…
instance-restart-tip = Neue Instanz aus der Historie dieser Instanz starten (Tab Modifizieren)
instance-actions = Aktionen
instance-activate = Instanz aktivieren
instance-suspend = Instanz anhalten
toast-instance-activated = Instanz aktiviert
toast-instance-suspended = Instanz angehalten
instance-retry-failed-jobs = { $n ->
    [one] { $n } fehlgeschlagenen Job wiederholen
   *[other] { $n } fehlgeschlagene Jobs wiederholen
}
instance-retry-failed-external = { $n ->
    [one] { $n } fehlgeschlagene externe Aufgabe wiederholen
   *[other] { $n } fehlgeschlagene externe Aufgaben wiederholen
}
instance-modify-tokens = Tokens modifizieren…
instance-delete = Instanz löschen…
instance-delete-title = Prozessinstanz löschen
instance-delete-body = Instanz { $id } { $bk } löschen?
    Alle ihre Tokens, Aufgaben und Jobs werden entfernt. Dies kann nicht rückgängig gemacht werden.
toast-instance-deleted = Instanz gelöscht
toast-instance-delete-failed = Instanz konnte nicht gelöscht werden
tokens-none = Keine aktiven Tokens
tokens-in = in { $path }
tokens-finished = Abgeschlossen — keine Tokens
scope-process = Prozess
scope-task = Aufgabe { $id }
scope-execution = Ausführung { $id }
vars-add = + Hinzufügen
vars-copy-json = JSON kopieren
vars-copy-json-tip = Alle Variablen als JSON-Objekt kopieren
toast-vars-copied = Variablen als JSON kopiert
vars-name-hint = Name
vars-date-format = Format: 2026-10-20T10:00:00.000+0300
vars-name-required = Name ist erforderlich
toast-var-restored = Variable { $name } wiederhergestellt
toast-undo-failed = Rückgängig fehlgeschlagen
toast-var-saved = Variable { $name } gespeichert
toast-var-save-failed = Variable { $name } konnte nicht gespeichert werden
vars-empty = Keine Variablen.
vars-no-match = Keine Variable passt zum Filter.
vars-row-tip = Klick zum Aufklappen · Doppelklick kopiert den Namen
vars-deser-error = Deserialisierungsfehler: { $error }
vars-delete-title = Variable löschen
vars-delete-body = Variable „{ $name }“ aus dieser Instanz löschen?
toast-var-deleted = Variable { $name } gelöscht
toast-var-delete-failed = { $name } konnte nicht gelöscht werden
vars-err-integer = keine Ganzzahl: { $error }
vars-err-boolean = true oder false verwenden
vars-err-date = Datum nicht erkannt
vars-err-json = ungültiges JSON: { $error }
tasks-finished = Instanz abgeschlossen — keine offenen Aufgaben.
tasks-empty = Keine offenen Benutzeraufgaben.
task-claim = Übernehmen
task-claim-tip = Als { $user } übernehmen
task-unclaim = Freigeben
toast-task-unassigned = Aufgabenzuweisung aufgehoben
toast-assign-failed = Bearbeiter konnte nicht gesetzt werden
toast-task-claimed = Aufgabe übernommen
toast-claim-failed = Übernehmen fehlgeschlagen
toast-task-unclaimed = Aufgabe freigegeben
toast-unclaim-failed = Freigeben fehlgeschlagen
task-due-plus-day = Fällig +1 Tag
task-clear-due = Fälligkeit löschen
toast-due-tomorrow = Fälligkeit auf morgen gesetzt
toast-due-failed = Fälligkeit konnte nicht gesetzt werden
toast-due-cleared = Fälligkeit gelöscht
toast-due-clear-failed = Fälligkeit konnte nicht gelöscht werden
toast-due-now = Fälligkeit auf jetzt gesetzt
toast-due-plus-hour = Fälligkeit um eine Stunde verschoben
task-complete = Abschließen…
task-complete-tip = Aufgabe ohne Variablen abschließen
task-complete-title = Aufgabe abschließen
task-complete-body = Aufgabe „{ $name }“ ohne Variablen abschließen? Der Prozess läuft weiter.
task-complete-button = Abschließen
toast-task-completed = Aufgabe abgeschlossen
toast-task-complete-failed = Aufgabe konnte nicht abgeschlossen werden
inc-resolved = gelöst
inc-deleted = gelöscht
inc-closed = geschlossen
inc-retry-1 = Wiederholen (Wiederholungen=1)
inc-retries-3 = Wiederholungen=3
inc-stack-trace = Stacktrace
inc-error-details = Fehlerdetails
trace-job-title = Stacktrace von Job { $id }
trace-external-title = Fehlerdetails der externen Aufgabe { $id }
inc-annotate = Anmerken…
inc-resolve = Lösen
inc-resolve-tip = Incident löschen (behebt die Ursache nicht)
toast-incident-resolved = Incident gelöst
toast-incident-resolve-failed = Incident konnte nicht gelöst werden
inc-annotation-title = Incident-Anmerkung
toast-annotation-saved = Anmerkung gespeichert
toast-annotation-failed = Anmerkung konnte nicht gespeichert werden
trace-load-failed = Laden fehlgeschlagen: { $error }
jobs-finished = Instanz abgeschlossen — keine Jobs.
jobs-empty = Keine Jobs (Timer oder asynchrone Fortsetzungen).
job-execute = Jetzt ausführen
job-execute-tip = Job synchron ausführen
toast-job-executed = Job ausgeführt
toast-job-execute-failed = Jobausführung fehlgeschlagen
job-retries-1 = Wiederholungen → 1
job-retries-3 = Wiederholungen → 3
job-due-now = Fällig jetzt
job-due-plus-hour = Fällig +1 h
toast-job-suspension = Job-Anhaltestatus geändert
toast-job-change-failed = Job konnte nicht geändert werden
ext-finished = Instanz abgeschlossen — keine externen Aufgaben.
ext-empty = Keine externen Aufgaben.
ext-not-locked = — (nicht gesperrt)
ext-never-failed = — (nie fehlgeschlagen)
ext-unlock = Entsperren
toast-ext-unlocked = Externe Aufgabe entsperrt
toast-ext-unlock-failed = Entsperren fehlgeschlagen
ext-priority-plus = Priorität +10
toast-priority-raised = Priorität erhöht
toast-priority-failed = Priorität konnte nicht gesetzt werden
called-empty = Keine aufgerufenen Prozessinstanzen.
timeline-empty = Noch keine Aktivitätshistorie.
timeline-start = Start { $when }
timeline-end = Ende  { $when }  ({ $duration })
timeline-active = noch aktiv
timeline-cancelled = abgebrochen
timeline-assignee = Bearbeiter { $user }
timeline-called = aufgerufene Instanz { $id }
mod-added-start-before = Hinzugefügt: Start vor { $name }
mod-added-start-after = Hinzugefügt: Start nach { $name }
mod-added-cancel = Hinzugefügt: Tokens abbrechen an { $name }
mod-move-label = Token verschieben von { $name }
mod-added-move = Hinzugefügt: Token verschieben { $from } → { $to }
mod-restart-title = Diese abgeschlossene Instanz neu starten
mod-restart-help = Wählen Sie, wo die neue Instanz beginnen soll: Rechtsklick auf ein Element im Diagramm → „Token davor/danach starten“. Variablen vom Ende dieser Instanz werden kopiert, wenn „Anfangsvariablen“ aus ist.
mod-title = Token-Modifikation
mod-help = Rechtsklick auf ein Element im Diagramm: Token davor/danach starten, Tokens dort abbrechen oder einen Token verschieben. Es wird nichts gesendet, bis Sie anwenden.
mod-empty = Noch keine Anweisungen.
mod-skip-listeners = Benutzerdefinierte Listener überspringen
mod-skip-io = I/O-Mappings überspringen
mod-annotation-hint = Anmerkung (optional, wird im Operationsprotokoll gespeichert)
mod-restart-button = Instanz neu starten…
mod-apply-button = Modifikation anwenden…
mod-restart-body = Neue Instanz dieses Prozesses starten mit:
    { $list }

    Variablen werden aus der abgeschlossenen Instanz kopiert.
mod-apply-body = Auf Instanz { $id } anwenden:
    { $list }
mod-restart-confirm-title = Instanz neu starten
mod-confirm-title = Tokens modifizieren
toast-restarted = Instanz neu gestartet (siehe Instanzliste der Definition)
toast-restart-failed = Neustart fehlgeschlagen
toast-modified = Modifikation angewendet
toast-modify-failed = Modifikation fehlgeschlagen

## Search place
search-syntax-title = Abfragesyntax
search-syntax-operators = Operatoren: feld:wert (=), != > >= < <= und ~ (like, * Platzhalter). Werte mit Leerzeichen in Anführungszeichen. Ein einzelnes Wort sucht Business-Keys; eine einzelne UUID ist eine Instanz-ID.
search-examples = Beispiele
search-preview-empty = Bewegen Sie den Zeiger über ein Ergebnis, um seine Variablen zu sehen.
search-variables = Variablen
search-add-column = + Spalte
search-add-column-tip = Als Ergebnisspalte hinzufügen
search-query-hint = def:key  bk:ABC*  var.amount>1000  act:Task_x  state:finished  …   (Enter sucht, Tab übernimmt einen Vorschlag)
search-suggest-hint = Tab übernimmt · Enter sucht
search-mode-runtime = Runtime
search-mode-history = Historie
search-mode-auto = auto
search-mode-tip = Welche API antwortet: Runtime = nur laufende Instanzen, Historie = laufende und abgeschlossene. Klicken zum Erzwingen.
search-save-query = Aktuelle Abfrage speichern…
search-no-saved = Keine gespeicherten Suchen
search-results = { $n ->
    [one] { $n } Ergebnis
   *[other] { $n } Ergebnisse
}
search-var-column-hint = + Variablenspalte
search-remove-column = Spalte entfernen
search-empty-hint = Abfrage eingeben und Enter drücken. Die Feldliste steht im Tab „Syntax“ im Dock.
search-no-match = Keine passenden Instanzen.
search-var-col = var { $name }
running-for = läuft seit { $duration }
search-selected = { $n } ausgewählt
search-clear-selection = Auswahl aufheben
batch-suspend-title = Instanzen anhalten
batch-suspend-body = { $n } Instanz(en) als Batch anhalten?
batch-activate-title = Instanzen aktivieren
batch-activate-body = { $n } Instanz(en) als Batch aktivieren?
batch-retry-jobs = Fehlgeschlagene Jobs wiederholen
batch-retry-title = Jobs wiederholen
batch-retry-body = Job-Wiederholungen für { $n } Instanz(en) als Batch auf 1 setzen?
batch-set-variable = Variable setzen…
batch-set-variable-hint = Geben Sie name=wert in das Variablenfeld ein und drücken Sie dort Enter, solange Zeilen ausgewählt sind (nur Zeichenketten).
batch-delete = Löschen…
batch-delete-instances-title = Instanzen löschen
batch-delete-instances-body = { $n } Prozessinstanz(en) als Batch löschen? Dies kann nicht rückgängig gemacht werden.
toast-batch-started = Batch { $id } gestartet ({ $jobs } Jobs). Siehe Batches (Strg+Umschalt+B).
toast-batch-start-failed = Batch konnte nicht gestartet werden

## Dock tabs
dock-inspector = Inspektor
dock-instances = Instanzen
dock-incidents = Incidents
dock-versions = Versionen
dock-overview = Übersicht
dock-inspect = Untersuchen
dock-variables = Variablen
dock-tasks = Aufgaben
dock-jobs = Jobs
dock-external = Extern
dock-called = Aufgerufen
dock-timeline = Zeitleiste
dock-modify = Modifizieren
dock-preview = Vorschau
dock-syntax = Syntax
dock-matches = Treffer

## Table columns
col-name = Name
col-key = Schlüssel
col-version = Version
col-running = Laufend
col-incidents = Incidents
col-state = Status
col-business-key = Business-Key
col-started = Gestartet
col-id = ID
col-deployed = Deployt
col-type = Typ
col-jobs = Jobs
col-ended = Beendet
col-time = Zeit
col-definition = Definition
col-activity = Aktivität
col-message = Meldung
col-instance = Instanz
col-ended-duration = Beendet / Dauer
col-business-key-id = Business-Key / ID
col-via = Über
col-flow = Fluss
col-target = Ziel
col-condition = Bedingung

## Inspector sections
section-identity = Identität
section-runtime = Laufzeit
section-in-this-instance = In dieser Instanz
section-execution = Ausführung
section-implementation = Implementierung
section-user-task = Benutzeraufgabe
section-form = Formular
section-event-definition = Ereignisdefinition
section-outgoing-flows = Ausgehende Flüsse
section-connections = Verbindungen
section-multi-instance = Mehrfachinstanz
section-input-output-mapping = Ein-/Ausgabe-Mapping
section-listeners = Listener
section-extension-properties = Erweiterungseigenschaften
section-other-attributes = Weitere Attribute
section-xml = XML
section-definition = Definition
section-instance = Instanz
section-called-by = Aufgerufen von
section-condition = Bedingung
section-now = Jetzt

## Key/value row labels
kv-id = ID
kv-name = Name
kv-key = Schlüssel
kv-type = Typ
kv-version = Version
kv-documentation = Dokumentation
kv-lane = Lane
kv-pool = Pool
kv-parent = übergeordnet
kv-attached-to = angeheftet an
kv-interrupting = unterbrechend
kv-boundary-event = Randereignis
kv-executed-history = ausgeführt (Historie)
kv-retry-time-cycle = Wiederholungszyklus
kv-called-element = aufgerufenes Element
kv-script-format = Skriptformat
kv-script = Skript
kv-message = Nachricht
kv-message-ref = Nachrichtenreferenz
kv-signal = Signal
kv-signal-ref = Signalreferenz
kv-error = Fehler
kv-error-code = Fehlercode
kv-code-variable = Codevariable
kv-message-variable = Nachrichtenvariable
kv-escalation = Eskalation
kv-escalation-code = Eskalationscode
kv-condition = Bedingung
kv-language = Sprache
kv-variable-name = Variablenname
kv-variable-events = Variablenereignisse
kv-link = Link
kv-compensates = kompensiert
kv-wait-for-completion = auf Abschluss warten
kv-default-flow = Standardfluss
kv-from = von
kv-to = nach
kv-mode = Modus
kv-collection = Sammlung
kv-element-variable = Elementvariable
kv-cardinality = Kardinalität
kv-completion-condition = Abschlussbedingung
kv-resource = Ressource
kv-deployment = Deployment
kv-history-ttl = Historien-TTL
kv-description = Beschreibung
kv-business-key = Business-Key
kv-definition = Definition
kv-definition-id = Definitions-ID
kv-state = Status
kv-started = gestartet
kv-ended = beendet
kv-duration = Dauer
kv-running-for = läuft seit
kv-started-by = gestartet von
kv-start-activity = Startaktivität
kv-delete-reason = Löschgrund
kv-super-instance = übergeordnete Instanz
kv-root-instance = Wurzelinstanz
kv-tenant = Mandant
kv-history-removal = Historienbereinigung
kv-task-id = Aufgaben-ID
kv-due = fällig
kv-follow-up = Wiedervorlage
kv-priority = Priorität
kv-assignee = Bearbeiter
kv-annotation = Anmerkung
kv-root-cause-incident = Ursprungs-Incident
kv-in-instance = in Instanz
kv-job-id = Job-ID
kv-retries = Wiederholungen
kv-configuration = Konfiguration
kv-exception = Ausnahme
kv-topic = Topic
kv-worker = Worker
kv-lock-expires = Sperre läuft ab
kv-scope = Bereich
kv-job-retries = Job-Wiederholungen
kv-job-error = Job-Fehler
kv-terminate = Terminierung
kv-cancel = Abbruch
kv-expression = Ausdruck
kv-default = Standard

## BPMN element kinds
bpmn-task = Aufgabe
bpmn-user-task = Benutzeraufgabe
bpmn-service-task = Serviceaufgabe
bpmn-script-task = Skriptaufgabe
bpmn-business-rule-task = Geschäftsregelaufgabe
bpmn-send-task = Sendeaufgabe
bpmn-receive-task = Empfangsaufgabe
bpmn-manual-task = Manuelle Aufgabe
bpmn-call-activity = Call Activity
bpmn-subprocess = Unterprozess
bpmn-ad-hoc-subprocess = Ad-hoc-Unterprozess
bpmn-transaction = Transaktion
bpmn-start-event = Startereignis
bpmn-end-event = Endereignis
bpmn-intermediate-catch-event = Eintretendes Zwischenereignis
bpmn-intermediate-throw-event = Auslösendes Zwischenereignis
bpmn-boundary-event = Randereignis
bpmn-exclusive-gateway = Exklusives Gateway
bpmn-parallel-gateway = Paralleles Gateway
bpmn-inclusive-gateway = Inklusives Gateway
bpmn-event-based-gateway = Ereignisbasiertes Gateway
bpmn-complex-gateway = Komplexes Gateway
bpmn-pool = Pool
bpmn-lane = Lane
bpmn-data-object = Datenobjekt
bpmn-data-store = Datenspeicher
bpmn-data-input = Dateneingabe
bpmn-data-output = Datenausgabe
bpmn-text-annotation = Textanmerkung
bpmn-group = Gruppe
bpmn-event-timer = Timer
bpmn-event-message = Nachricht
bpmn-event-signal = Signal
bpmn-event-error = Fehler
bpmn-event-escalation = Eskalation
bpmn-event-conditional = Bedingung
bpmn-event-link = Link
bpmn-event-compensation = Kompensation
bpmn-event-terminate = Terminierung
bpmn-event-cancel = Abbruch
bpmn-flow-sequence-flow = Sequenzfluss
bpmn-flow-message-flow = Nachrichtenfluss
bpmn-flow-association = Assoziation
bpmn-flow-data-input-association = Dateneingabe-Assoziation
bpmn-flow-data-output-association = Datenausgabe-Assoziation

## Stragglers
palette-hint = Befehl, Definitionsschlüssel oder -name, Instanz-ID oder Business-Key eingeben…   (> Befehle, # Business-Key)
connect-url-hint = http://host:8080/engine-rest  (eine Cockpit-URL geht auch)
inc-resolve-title = Incident lösen
inc-resolve-body = Diesen Incident als gelöst markieren? Der zugrunde liegende Job oder die Aufgabe bleibt unberührt.
definitions-filter-hint = Nach Schlüssel oder Name filtern

## Time
time-ago = vor { $d }
time-in = in { $d }
time-just-now = gerade eben
dur-ms = { $ms } ms
dur-s = { $s } s
dur-m-s = { $m } min { $s } s
dur-h-m = { $h } h { $m } min
dur-d-h = { $d } d { $h } h
bpmn-event-generic = Ereignis

## Implementation / user-task attribute labels
kv-class = Klasse
kv-delegate-expression = Delegate-Ausdruck
kv-result-variable = Ergebnisvariable
kv-task-priority = Aufgabenpriorität
kv-decision-ref = Entscheidungsreferenz
kv-decision-binding = Entscheidungsbindung
kv-decision-version = Entscheidungsversion
kv-decision-tenant = Entscheidungsmandant
kv-map-decision-result = Entscheidungsergebnis abbilden
kv-called-binding = Aufrufbindung
kv-called-version = Aufrufversion
kv-called-version-tag = Aufruf-Versionstag
kv-called-tenant = Aufrufmandant
kv-variable-mapping-class = Variablen-Mapping-Klasse
kv-variable-mapping-delegate = Variablen-Mapping-Delegate
kv-candidate-users = Kandidaten (Benutzer)
kv-candidate-groups = Kandidaten (Gruppen)
kv-due-date = Fälligkeitsdatum
kv-form-key = Formularschlüssel
kv-form-ref = Formularreferenz
kv-form-binding = Formularbindung
kv-form-version = Formularversion

## Documentation blocks
doc-show-all = Alles zeigen ({ $n } weitere Zeilen)
doc-show-less = Weniger zeigen
kv-executable = ausführbar
section-process = Prozess

## Opening files, document sources
cmd-open-file = BPMN-Datei öffnen…
cmd-view-xml = XML anzeigen
cmd-compare = BPMN vergleichen…
open-file-title = BPMN-Datei öffnen
file-needs-engine = Braucht eine verbundene Engine
source-offline = Nicht mit einer Engine verbunden
source-no-data-dir = Kein Datenverzeichnis zum Speichern der Definition
settings-xml-editor = Externer XML-Editor
settings-xml-editor-hint = Befehl für „XML im Editor öffnen“, z. B. code -g {"{"}file{"}"}:{"{"}line{"}"}:{"{"}col{"}"} oder kate -l {"{"}line{"}"} -c {"{"}col{"}"} {"{"}file{"}"}. Leer blendet den Eintrag aus.

## XML view
ctx-view-xml = XML anzeigen
ctx-open-xml-with = XML öffnen mit…
ctx-open-xml-editor = XML im Editor öffnen
ctx-compare-with = Vergleichen mit…
ctx-copy-location = Pfad:Zeile:Spalte kopieren
ctx-copy-location-tip = Datei und Position dieses Elements in der Form, die Editoren wie VS Code verstehen (code -g pfad:zeile:spalte). Engine-Definitionen werden zuerst ins Datenverzeichnis gespeichert.
xml-find = Suchen
xml-find-hint = Im XML suchen  (Enter weiter, Umschalt+Enter zurück)
xml-fold = { $n ->
    [one] { $n } unveränderte Zeile
   *[other] { $n } unveränderte Zeilen
}
xml-identical = identisch
xml-whole-file = Ganze Datei
xml-whole-file-tip = Unveränderte Zeilen zeigen statt sie einzuklappen
xml-prev-change-tip = Vorherige Änderung (Umschalt+N)
xml-next-change-tip = Nächste Änderung (N)
xml-clear-highlight-tip = Hervorhebung entfernen (Esc)
xml-show-diagram = Diagramm zeigen
xml-show-diagram-tip = Zum Diagramm wechseln und das hervorgehobene Element auswählen
xml-open-with = Öffnen mit…
xml-open-with-tip = Ein Programm in der Anwendungsauswahl des Systems wählen
xml-open-editor = Im Editor öffnen
xml-load-failed = Dokument kann nicht geladen werden: { $error }
xml-export-failed = XML kann nicht gespeichert werden
xml-editor-failed = Editor kann nicht gestartet werden
xml-open-with-failed = Die Anwendungsauswahl ist fehlgeschlagen
xml-not-loaded = Das Dokument ist noch nicht geladen
xml-ctx-show-in-diagram = Im Diagramm zeigen
xml-ctx-highlight-element = Dieses Element hervorheben
xml-ctx-copy-line = Zeile kopieren
hint-xml = Strg+F suchen · F3 nächster Treffer · Esc Hervorhebung entfernen · Rechtsklick auf eine Zeile: im Diagramm zeigen, Pfad:Zeile:Spalte kopieren
hint-xml-diff = N / Umschalt+N nächste / vorherige Änderung · Strg+F suchen · Esc Hervorhebung entfernen · Klick auf ⋯ klappt auf

## Diff view
diff-before = Vorher
diff-after = Nachher
diff-choose = wählen…
diff-swap-tip = Vorher und Nachher tauschen
diff-mode-unified = Zusammen
diff-mode-before = Vorher
diff-mode-after = Nachher
diff-mode-side-by-side = Nebeneinander
diff-added = +{ $n } hinzugefügt
diff-removed = −{ $n } entfernt
diff-changed = ~{ $n } geändert
diff-moved = ↔{ $n } verschoben
diff-filter-tip = Diese Art von Änderung zeigen oder ausblenden
diff-no-changes = Keine Unterschiede
diff-all-filtered = Alle Änderungen sind ausgefiltert
diff-dim-unchanged = Unverändertes abblenden
diff-xml = XML-Vergleich
diff-next-tip = Nächste Änderung (N)
diff-prev-tip = Vorherige Änderung (Umschalt+N)
diff-pick-file = Lokale Datei…
diff-pick-file-title = BPMN-Datei zum Vergleichen wählen
diff-path-hint = oder Pfad eingeben
diff-use-path = Übernehmen
diff-no-such-file = Datei nicht gefunden
diff-recent = Geöffnete und letzte Dokumente
diff-engine-definitions = Definitionen der Engine
diff-filter-hint = nach Schlüssel oder Name filtern
diff-setup-title = Zwei BPMN-Diagramme vergleichen
diff-setup-body = Version vorher und nachher wählen: lokale Dateien oder auf der Engine bereitgestellte Definitionen.
diff-status-added = Hinzugefügt
diff-status-removed = Entfernt
diff-status-changed = Geändert
diff-status-moved = Verschoben (nur Layout)
diff-also-moved = Auch verschoben
diff-only-layout = Nur Position oder Größe hat sich geändert.
diff-id-changed = frühere id: { $old }
diff-more-changes = …und { $n } weitere
section-document = Dokument
section-changes = Änderungen
dock-changes = Änderungen
hint-diff = N / Umschalt+N nächste / vorherige Änderung · Klick auf eine Änderung springt hin · Strg+U XML-Vergleich · Rechtsklick: XML eines Elements
