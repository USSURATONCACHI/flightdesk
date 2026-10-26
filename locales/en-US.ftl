# Flightdeck — English (United States)
# Language packs are Fluent files named after their BCP 47 tag; see docs for how to add one.
language-name = English

## Common words
common-remove = Remove
common-reset = reset
common-on = On
common-off = Off
common-enabled = Enabled
common-none = none
common-version = version
common-cancel = Cancel
common-close = Close
common-copy = Copy
common-copied = copied
common-details = Details
common-yes = Yes
common-no = No
common-ok = OK
common-loading = loading…
common-error = Error
common-retry = Retry
common-search = Search
common-open = Open
common-open-new-tab = Open in new tab
common-refresh = Refresh
common-all = all
common-unknown = unknown

## Themes & density
theme-system = System
theme-light = Light
theme-dark = Dark
density-compact = Compact
density-normal = Normal
density-comfortable = Comfortable

## Connection fields (shared by the connect screen and settings)
connect-name = Name
connect-rest-url = REST URL
connect-user = User
connect-password = Password
connect-remember = remember
connect-cockpit-url = Cockpit URL
connect-cockpit-url-hint = Used by “Copy Cockpit link”. Leave empty to derive it from the REST URL.
connect-allow-self-signed = Allow self-signed TLS
connect-connect = Connect
connect-connected = connected
connect-add = + Add connection
connect-default-name = engine { $n }

## Settings place
settings-title = Settings
settings-saved-automatically = saved automatically
settings-connections = Connections
settings-auto-connect = Connect to the last engine on start-up
settings-appearance = Appearance
settings-theme = Theme
settings-language = Language
settings-language-system = System
settings-language-none = no language packs found; message ids are shown
settings-language-warnings = { $n ->
    [one] { $n } problem in the language pack (hover for details)
   *[other] { $n } problems in the language pack (hover for details)
}
settings-density = Density
settings-diagram-input = Diagram input
settings-mouse-wheel = Mouse wheel
settings-wheel-zooms = zooms (Ctrl+wheel pans)
settings-wheel-pans = pans (Ctrl+wheel zooms)
settings-inertia = Inertia after flicking the diagram
settings-zoom-speed = Zoom speed
settings-reset-to = back to { $value }×
settings-zoom-global = Global
settings-zoom-global-tip = Scales every zoom gesture: wheel, pinch, the zoom dial and the keyboard
settings-zoom-mouse = Mouse
settings-zoom-mouse-tip = Extra factor for pointer gestures (wheel, pinch, zoom dial), on top of Global
settings-zoom-keyboard = Keyboard
settings-zoom-keyboard-tip = Extra factor for + / − keys, on top of Global
settings-zoom-alt = Alt held
settings-zoom-alt-tip = Pan and zoom speed while Alt is held, on top of everything else. Below 1 slows down for fine work
settings-zoom-effective = effective: mouse { $mouse }×, keyboard { $keyboard }×, with Alt ×{ $alt } more
settings-unlimited-drag = Unlimited drag
settings-unlimited-drag-on-tip = Teleport the pointer to the opposite edge when a drag reaches the view border
settings-unlimited-drag-off-tip = Drags stop at the edge
settings-find = Find in diagram (Ctrl+F)
settings-find-instances = Also search running instances and highlight the activities they wait at
settings-find-business-keys = by business key (contains)
settings-find-ids = by process instance id (contains)
settings-refresh = Refresh
settings-refresh-instances = Instance views (seconds)
settings-refresh-lists = Lists (seconds)
settings-page-size = Page size
settings-pause-unfocused = Pause auto-refresh while the window is not focused
settings-confirm-destructive = Confirm destructive actions
settings-mcp = MCP server (AI agents)
settings-mcp-description = Exposes Flightdeck to AI agents over the Model Context Protocol (Streamable HTTP on localhost). Tools cover the engine (definitions, instances, variables, incidents, jobs, tasks, modification, batches) and the UI itself (navigate, select, zoom, dock, find, screenshots, keys).
settings-mcp-port = Port
settings-mcp-token = Bearer token (optional)
settings-mcp-listening = listening on { $url }
settings-mcp-copy-url = Copy URL
settings-mcp-copy-config = Copy Claude Code config
settings-mcp-stats = { $requests } requests · { $calls } tool calls · { $errors } errors
settings-mcp-last = last: { $tool }
settings-saved-searches = Saved searches
settings-saved-searches-empty = Save a query from the Search place (☆).
settings-about = About
settings-about-text = Flightdeck — a fast, keyboard-first cockpit for Camunda 7.
settings-file = settings
settings-not-persisted = settings are not persisted in this session

## Connection screen
connect-title = Connections
connect-list-hint = Click · Ctrl adds · Shift ranges · Alt removes · drag to the bin
connect-unnamed = (unnamed)
connect-add-tip = Add a connection
connect-new-name = Connection { $n }
connect-empty = No saved connections yet.
connect-bin-release = Release to delete
connect-bin-hint = Drop connections here to delete
connect-delete-one = Delete the connection “{ $name }”?
connect-delete-many = Delete { $n } connections?
    • { $list }
connect-delete-title = Delete connections
connect-deleted = { $n ->
    [one] Deleted { $n } connection
   *[other] Deleted { $n } connections
}
connect-subtitle = Connect to a Camunda 7 engine
connect-select-hint = Select a connection on the left, or add one with “+”.
connect-user-hint = optional (basic auth)
connect-show-password = Show password
connect-remember-password = Remember password
connect-remember-tip = Stored (owner-readable only) in the settings file
connect-connecting = Connecting…
connect-url-note = The URL is normalised: `host:8080`, `…/camunda/app/cockpit/…` and `…/engine-rest` all work.
connect-open-bpmn = Open a .bpmn file
connect-open-bpmn-tip = Offline viewer: no engine needed (Ctrl+O; you can also drop files onto the window)
common-delete = Delete
common-suspend = Suspend
common-activate = Activate

## Toasts and errors
toast-connected = Connected to Camunda { $version }
error-auth = Authentication failed: check user and password.
error-not-camunda = 404 at /version: the URL does not point at a Camunda REST API.
toast-mcp-listening = MCP server listening on { $url }
toast-mcp-failed = MCP server could not start on port { $port }
toast-mcp-stopped = MCP server stopped
toast-no-definition-key = No definition with key { $key }
toast-copied = Copied { $what }
toast-job-retries = Job retries set to { $n }
toast-job-retries-failed = Could not set job retries
toast-external-retries = External task retries set to { $n }
toast-external-retries-failed = Could not set external task retries
definition-suspend-title = Suspend definition
definition-suspend-body = Suspend process definition { $id } including all its running instances?
definition-activate-title = Activate definition
definition-activate-body = Activate process definition { $id } including all its running instances?
toast-definition-suspended = Definition suspended
toast-definition-activated = Definition activated
toast-suspension-failed = Could not change suspension state
perms-warning = { $n ->
    [one] Settings file is readable by other users ({ $n } path). Saved passwords should be private.
   *[other] Settings files are readable by other users ({ $n } paths). Saved passwords should be private.
}
perms-fix = Fix permissions
perms-fixed = Settings directory is now private (700 / 600)
perms-fix-failed = Some permissions could not be changed

## Top bar, status bar, tabs
status-connecting = connecting…
status-offline = offline
topbar-refresh-everything = Refresh everything
topbar-disconnect = Disconnect
tab-close-tip = Close (Ctrl+W)
tab-new-tip = New tab (Ctrl+T)
topbar-shortcuts-tip = Shortcuts (?)
topbar-mcp-off = MCP off
topbar-mcp-tip = Built-in MCP server for AI agents. Click to toggle.
topbar-settings-tip = Settings (Ctrl+,)
topbar-theme-tip = Theme: system / light / dark (Ctrl+Shift+T cycles)
topbar-search = Search or jump…
nav-forward-tip = Forward (Alt+→)
nav-back-tip = Back (Alt+←)
refresh-resume-tip = Resume auto-refresh (Ctrl+Shift+P)
refresh-pause-tip = Pause auto-refresh (Ctrl+Shift+P)
refresh-now-tip = Refresh now (R / F5)
refresh-updating = updating…
refresh-updated = updated { $ago }
status-requests = { $n ->
    [one] { $n } request
   *[other] { $n } requests
}
status-paused = auto-refresh paused
status-paused-unfocused = refresh paused while unfocused

## Diagram context menu
ctx-inspect = Inspect
ctx-copy-id = Copy id
ctx-copy-name = Copy name
ctx-instances-waiting = Instances waiting here
ctx-instances-executed = Instances that executed this…
ctx-open-called = Open called process { $key }
ctx-go-source = Go to source: { $name }
ctx-go-target = Go to target: { $name }
ctx-diagram = Diagram
ctx-fit-diagram = Fit diagram

## Commands (palette, help)
cat-diagram = Diagram
cat-general = General
cat-navigate = Navigate
cat-tabs = Tabs
cat-view = View
cmd-back = Back
cmd-close-tab = Close tab
cmd-copy-cockpit-link = Copy link for Camunda Cockpit
cmd-copy-place-id = Copy current id (definition / instance)
cmd-disconnect = Disconnect from engine
cmd-find-in-diagram = Find element in diagram
cmd-fit-diagram = Fit diagram
cmd-focus-filter = Focus filter / search field
cmd-forward = Forward
cmd-go-batches = Go to batches
cmd-go-definitions = Go to process definitions
cmd-go-incidents = Go to incidents
cmd-help = Keyboard shortcuts
cmd-new-tab = New tab
cmd-next-tab = Next tab
cmd-open-settings = Open settings
cmd-palette = Command palette
cmd-prev-tab = Previous tab
cmd-refresh = Refresh now
cmd-search-instances = Search process instances
cmd-toggle-dock = Toggle side dock
cmd-toggle-pause = Pause / resume auto-refresh
cmd-toggle-theme = Cycle theme: system → light → dark
cmd-zoom-in = Zoom in
cmd-zoom-out = Zoom out
help-d-alt = pan and zoom faster (×2 by default, see Settings)
help-d-alt-arrows = follow outgoing / incoming flow (1-9 picks)
help-d-arrows = pan (hold to accelerate, Shift faster)
help-d-copy = copy element id
help-d-double-click = zoom to element (empty: fit all)
help-d-drag = pan (flick for inertia)
help-d-enter = inspect · drill into call activity
help-d-esc = close find → clear selection
help-d-find = find element or instance (Enter/Shift+Enter cycle)
help-d-fit = fit all · 100% · fit selection · fit tokens
help-d-plus-minus = zoom in / out (Alt: by the Alt factor)
help-d-right-click = context menu
help-d-shift-wheel = pan horizontally
help-d-tab = next / previous element along the flow
help-d-toggles = minimap · grid · element ids
help-d-wheel = zoom at cursor

## Palette
palette-open-instance = Open instance { $id }
palette-kind-instance-id = process instance id
palette-open-as-definition = Open as definition id
palette-open-definition = Open definition { $id }
palette-kind-definition-id = process definition id
palette-search-business-key = Search instances for business key “{ $q }”
palette-search-instances = Search instances: { $q }
common-no-matches = No matches
toast-refresh-paused = Auto-refresh paused
toast-refresh-resumed = Auto-refresh resumed

## Help overlay
help-title = Keyboard & mouse
help-diagram = Diagram
help-everywhere = Everywhere
help-close-hint = Press ? or Esc to close
help-g-dock-tab = dock tab
help-g-tables = tables: move · open · open in new tab
help-g-middle-click = open in new tab
help-g-slash = focus the filter box
help-g-alt = inspect element under the cursor

## Diagram: find bar, zoom, tooltips
pick-banner = { $label } — click a target element, Esc to cancel
find-hint = Find element, business key or instance id…
find-no-matches = no matches
find-prev-tip = Previous match (Shift+Enter)
find-next-tip = Next match (Enter)
find-close-tip = Close (Esc)
find-case-tip = Case sensitive
find-flows-tip = Include sequence flows
find-cat-task = task
find-cat-gateway = gw
find-cat-event = ev
find-cat-call = call
find-cat-subprocess = sub
find-only-task = Only tasks
find-only-gateway = Only gateways
find-only-event = Only events
find-only-call = Only call activities
find-only-subprocess = Only sub-processes
find-count-elements = { $n ->
    [one] { $n } element
   *[other] { $n } elements
}
find-searching-instances = searching instances…
find-count-instances = { $n ->
    [one] { $n } instance
   *[other] { $n } instances
} at { $a ->
    [one] { $a } activity
   *[other] { $a } activities
}
find-legend-instance-short = instance match
find-go-to = Go to { $name }
zoom-tip = Zoom. Click for presets. Wheel zooms, Shift+wheel pans, 0 fits, 1 = 100%
zoom-fit-diagram = Fit diagram
zoom-fit-selection = Fit selection
zoom-minimap = Minimap
zoom-grid = Grid
zoom-ids = Element ids
zoom-dial-tip = Drag left or right to zoom, as far as you like
tip-running = { $n } running
tip-incidents = { $n ->
    [one] { $n } incident
   *[other] { $n } incidents
}
tip-executed = executed ×{ $n }
tip-timer = timer { $t }
tip-default-flow = default flow
find-explain-element = Element matches: { $field } “{ $snippet }”
find-explain-instances = { $n ->
    [one] { $n } running instance here matches:
   *[other] { $n } running instances here match:
}
find-explain-business-key = business key { $key }
find-explain-id = id { $id }
find-explain-join = A token of this instance is parked at this join, waiting for the other branch(es) to arrive.
find-explain-event-gateway = The instance is waiting here for one of the following events.
find-explain-more = … and { $n } more
find-explain-striped = striped: both the element and an instance match
find-explain-current = Current match { $i } of { $n } (Enter / Shift+Enter cycle)
find-legend-element = element text matches ({ $n })
find-legend-instance = a running instance here matches ({ $n })
find-legend-both = both ({ $n })
find-legend-current = current match · Enter cycles

## Places, lists and hints
place-definitions = Definitions
place-incidents = Incidents
place-batches = Batches
definitions-title = Process definitions
definitions-load-failed = Cannot load definitions: { $error }
definitions-empty = No process definitions deployed.
definitions-copy-key = Copy key
definitions-copy-id = Copy definition id
common-all-versions = All versions
common-nothing-matches-filter = Nothing matches the filter.
common-search-instances = Search instances
common-copy-business-key = Copy business key
common-open-definition = Open definition
common-select-on-diagram = Select on diagram
common-click-to-copy = Click to copy
common-find = Find
common-save = Save
common-set = Set
common-edit = Edit
common-clear = Clear
common-apply = Apply
common-restart = Restart
common-resume = Resume
common-prev = prev
common-next = next
common-now = now
common-undo = Undo
state-suspended = suspended
state-running-lower = running
n-running = { $n } running
n-incidents = { $n ->
    [one] { $n } incident
   *[other] { $n } incidents
}
n-tokens = { $n ->
    [one] { $n } token
   *[other] { $n } tokens
}
n-open = { $n } open
n-days = { $n ->
    [one] { $n } day
   *[other] { $n } days
}
hint-list = ↑↓ move · Enter open · Shift+Enter new tab · / filter
hint-definition = Wheel zoom · drag pan · Tab next element · Enter inspect / drill into call activity · click a badge to filter instances · Ctrl+F find · ? help
hint-instance = Shift+F fit tokens · Enter inspect / drill into called instance · right-click: modify tokens · Ctrl+1…9 dock tabs · ? help
hint-pick = Pick a target element on the diagram · Esc cancels
hint-incidents = ↑↓ move · Enter open instance · Shift+Enter new tab · / filter
hint-search = ↑↓ move · Enter open · Shift+Enter new tab · Space select · Ctrl+A all · double-click open · / query
hint-file = Drag/wheel: pan & zoom · 0 fit · F fit selection · Tab next element · Ctrl+F find · ? help
definition-show-instances = Show instances
definition-find-element = Find element
definition-search-instances-tip = Open the instance search for this definition
diagram-load-failed = Cannot load diagram: { $error }
diagram-loading = Loading diagram…
inspector-empty-definition = Select an element to inspect it.
    Tab cycles elements · Ctrl+F finds · hold Alt to inspect on hover.
inspector-empty-instance = Select an element to inspect it.
    Tab cycles elements · hold Alt to inspect on hover.
inspector-empty-file = Select an element to inspect it.
    Tab cycles elements, Ctrl+F finds them.
toast-element-id-copied = Element id copied
instances-at = at
instances-clear-activity-filter = Clear activity filter
instances-empty = No running instances match.
definition-no-incidents = No open incidents for this version.
instance-short = instance { $id }
incident-retry-job-tip = Set job retries to 1
incident-retry-external-tip = Set external task retries to 1
incident-retry-tip = Set retries to 1
incidents-group-by-message = Group by message
incidents-filter-hint = Filter message, type, definition, activity
incidents-empty = No open incidents. 🎉
incidents-none = No incidents.
batches-running = Running
batches-finished = Finished (history)
batches-empty-running = No running batches.
batches-empty-finished = No finished batches.
batch-delete-title = Delete batch
batch-delete-body = Delete this batch and its remaining jobs? Already executed jobs are not undone.
toast-batch-deleted = Batch deleted
toast-batch-delete-failed = Could not delete batch
toast-batch-updated = Batch updated
toast-batch-update-failed = Could not update batch
started-when = started { $when }
created-when = created { $when }
created-by = created by { $user }
file-diagram-warnings = { $n ->
    [one] { $n } diagram warning (see the Inspector tab)
   *[other] { $n } diagram warnings (see the Inspector tab)
}
file-open-failed = Cannot open { $path }: { $error }
inspector-running-here = { $n } running here
inspector-running-here-tip = Show instances waiting at this element
inspector-executed-tip = History search: executed activity
mi-sequential = sequential
mi-parallel = parallel
io-inputs = Inputs
io-outputs = Outputs
inspector-copy-xml = Copy XML
confirm-hint = Tab to Confirm, or type y
toast-hide-details = Hide details

## Instance place
instance-tab-title = inst { $id }
job-failed = failed
instance-super-tip = Super instance { $id }
instance-copy-bk-tip = Click to copy business key
toast-instance-id-copied = Instance id copied
instance-tokens-tip = Shift+F fits the camera to the tokens
instance-not-found = Instance not found (deleted, or history removed)
find-element-tip = Find element (Ctrl+F)
instance-loading = Loading instance…
trace-filter-hint = filter lines
extra-task = task: { $who }
assigned-to = assigned to { $user }
task-unassigned = unassigned
due-when = due { $when }
extra-job = job: due { $due } · retries { $retries }
extra-external = external: topic { $topic } · { $lock }
locked-by = locked by { $worker }
ext-unlocked = unlocked
ctx-modify = Modify
mod-start-before = Start token before
mod-start-after = Start token after
mod-cancel-here = Cancel tokens here
mod-move-from = Move token from here to…
token = token
token-async = async continuation
incident-of-type = incident ({ $type })
instance-restart = Restart…
instance-restart-tip = Start a new instance from the history of this one (Modify tab)
instance-actions = Actions
instance-activate = Activate instance
instance-suspend = Suspend instance
toast-instance-activated = Instance activated
toast-instance-suspended = Instance suspended
instance-retry-failed-jobs = { $n ->
    [one] Retry { $n } failed job
   *[other] Retry { $n } failed jobs
}
instance-retry-failed-external = { $n ->
    [one] Retry { $n } failed external task
   *[other] Retry { $n } failed external tasks
}
instance-modify-tokens = Modify tokens…
instance-delete = Delete instance…
instance-delete-title = Delete process instance
instance-delete-body = Delete instance { $id } { $bk }?
    All its tokens, tasks and jobs are removed. This cannot be undone.
toast-instance-deleted = Instance deleted
toast-instance-delete-failed = Could not delete instance
tokens-none = No active tokens
tokens-in = in { $path }
tokens-finished = Finished — no tokens
scope-process = process
scope-task = task { $id }
scope-execution = execution { $id }
vars-add = + Add
vars-copy-json = Copy JSON
vars-copy-json-tip = Copy all variables as a JSON object
toast-vars-copied = Variables copied as JSON
vars-name-hint = name
vars-date-format = format: 2026-10-20T10:00:00.000+0300
vars-name-required = Name is required
toast-var-restored = Variable { $name } restored
toast-undo-failed = Undo failed
toast-var-saved = Variable { $name } saved
toast-var-save-failed = Could not save variable { $name }
vars-empty = No variables.
vars-no-match = No variable matches the filter.
vars-row-tip = Click to expand · double-click to copy name
vars-deser-error = deserialisation error: { $error }
vars-delete-title = Delete variable
vars-delete-body = Delete variable “{ $name }” from this instance?
toast-var-deleted = Variable { $name } deleted
toast-var-delete-failed = Could not delete { $name }
vars-err-integer = not an integer: { $error }
vars-err-boolean = use true or false
vars-err-date = unrecognised date
vars-err-json = invalid JSON: { $error }
tasks-finished = Finished instance — no open tasks.
tasks-empty = No open user tasks.
task-claim = Claim
task-claim-tip = Claim as { $user }
task-unclaim = Unclaim
toast-task-unassigned = Task unassigned
toast-assign-failed = Could not set assignee
toast-task-claimed = Task claimed
toast-claim-failed = Could not claim
toast-task-unclaimed = Task unclaimed
toast-unclaim-failed = Could not unclaim
task-due-plus-day = Due +1 day
task-clear-due = Clear due
toast-due-tomorrow = Due date set to tomorrow
toast-due-failed = Could not set due date
toast-due-cleared = Due date cleared
toast-due-clear-failed = Could not clear due date
toast-due-now = Due date set to now
toast-due-plus-hour = Due date moved by one hour
task-complete = Complete…
task-complete-tip = Complete the task without variables
task-complete-title = Complete task
task-complete-body = Complete task “{ $name }” with no variables? The process continues.
task-complete-button = Complete
toast-task-completed = Task completed
toast-task-complete-failed = Could not complete task
inc-resolved = resolved
inc-deleted = deleted
inc-closed = closed
inc-retry-1 = Retry (retries=1)
inc-retries-3 = Retries=3
inc-stack-trace = Stack trace
inc-error-details = Error details
trace-job-title = Job { $id } stack trace
trace-external-title = External task { $id } error details
inc-annotate = Annotate…
inc-resolve = Resolve
inc-resolve-tip = Delete the incident (does not fix the cause)
toast-incident-resolved = Incident resolved
toast-incident-resolve-failed = Could not resolve incident
inc-annotation-title = Incident annotation
toast-annotation-saved = Annotation saved
toast-annotation-failed = Could not save annotation
trace-load-failed = Could not load: { $error }
jobs-finished = Finished instance — no jobs.
jobs-empty = No jobs (timers or async continuations).
job-execute = Execute now
job-execute-tip = Run the job synchronously
toast-job-executed = Job executed
toast-job-execute-failed = Job execution failed
job-retries-1 = Retries → 1
job-retries-3 = Retries → 3
job-due-now = Due now
job-due-plus-hour = Due +1 h
toast-job-suspension = Job suspension changed
toast-job-change-failed = Could not change job
ext-finished = Finished instance — no external tasks.
ext-empty = No external tasks.
ext-not-locked = — (not locked)
ext-never-failed = — (never failed)
ext-unlock = Unlock
toast-ext-unlocked = External task unlocked
toast-ext-unlock-failed = Could not unlock
ext-priority-plus = Priority +10
toast-priority-raised = Priority raised
toast-priority-failed = Could not set priority
called-empty = No called process instances.
timeline-empty = No activity history yet.
timeline-start = start { $when }
timeline-end = end   { $when }  ({ $duration })
timeline-active = still active
timeline-cancelled = cancelled
timeline-assignee = assignee { $user }
timeline-called = called instance { $id }
mod-added-start-before = Added: start before { $name }
mod-added-start-after = Added: start after { $name }
mod-added-cancel = Added: cancel tokens at { $name }
mod-move-label = Move token from { $name }
mod-added-move = Added: move token { $from } → { $to }
mod-restart-title = Restart this finished instance
mod-restart-help = Pick where the new instance should start: right-click an element on the diagram → “Start token before/after”. Variables from the end of this instance are copied when “initial variables” is off.
mod-title = Token modification
mod-help = Right-click an element on the diagram: start a token before/after it, cancel tokens there, or move a token. Nothing is sent until you apply.
mod-empty = No instructions yet.
mod-skip-listeners = Skip custom listeners
mod-skip-io = Skip I/O mappings
mod-annotation-hint = annotation (optional, stored in the operation log)
mod-restart-button = Restart instance…
mod-apply-button = Apply modification…
mod-restart-body = Start a new instance of this process with:
    { $list }

    Variables are copied from the finished instance.
mod-apply-body = Apply to instance { $id }:
    { $list }
mod-restart-confirm-title = Restart instance
mod-confirm-title = Modify tokens
toast-restarted = Instance restarted (see the definition's instance list)
toast-restart-failed = Restart failed
toast-modified = Modification applied
toast-modify-failed = Modification failed

## Search place
search-syntax-title = Query syntax
search-syntax-operators = Operators: field:value (=), != > >= < <= and ~ (like, * wildcard). Quote values with spaces. A bare word searches business keys; a bare uuid is an instance id.
search-examples = Examples
search-preview-empty = Move the cursor over a result to preview its variables.
search-variables = Variables
search-add-column = + col
search-add-column-tip = Add as a result column
search-query-hint = def:key  bk:ABC*  var.amount>1000  act:Task_x  state:finished  …   (Enter to search, Tab to accept a suggestion)
search-suggest-hint = Tab accepts · Enter searches
search-mode-runtime = runtime
search-mode-history = history
search-mode-auto = auto
search-mode-tip = Which API answers: runtime = running instances only, history = running and finished. Click to force.
search-save-query = Save current query…
search-no-saved = No saved searches
search-results = { $n ->
    [one] { $n } result
   *[other] { $n } results
}
search-var-column-hint = + variable column
search-remove-column = Remove column
search-empty-hint = Type a query and press Enter. Open the Syntax tab in the dock for the field list.
search-no-match = No instances match.
search-var-col = var { $name }
running-for = running { $duration }
search-selected = { $n } selected
search-clear-selection = Clear selection
batch-suspend-title = Suspend instances
batch-suspend-body = Suspend { $n } instance(s) as a batch?
batch-activate-title = Activate instances
batch-activate-body = Activate { $n } instance(s) as a batch?
batch-retry-jobs = Retry failed jobs
batch-retry-title = Retry jobs
batch-retry-body = Set job retries to 1 for { $n } instance(s) as a batch?
batch-set-variable = Set variable…
batch-set-variable-hint = Type name=value in the variable field, then press Enter in it while rows are selected (strings only).
batch-delete = Delete…
batch-delete-instances-title = Delete instances
batch-delete-instances-body = Delete { $n } process instance(s) as a batch? This cannot be undone.
toast-batch-started = Batch { $id } started ({ $jobs } jobs). See Batches (Ctrl+Shift+B).
toast-batch-start-failed = Batch could not be started

## Dock tabs (English names remain the MCP identifiers)
dock-inspector = Inspector
dock-instances = Instances
dock-incidents = Incidents
dock-versions = Versions
dock-overview = Overview
dock-inspect = Inspect
dock-variables = Variables
dock-tasks = Tasks
dock-jobs = Jobs
dock-external = External
dock-called = Called
dock-timeline = Timeline
dock-modify = Modify
dock-preview = Preview
dock-syntax = Syntax
dock-matches = Matches

## Table columns
col-name = Name
col-key = Key
col-version = Version
col-running = Running
col-incidents = Incidents
col-state = State
col-business-key = Business key
col-started = Started
col-id = Id
col-deployed = Deployed
col-type = Type
col-jobs = Jobs
col-ended = Ended
col-time = Time
col-definition = Definition
col-activity = Activity
col-message = Message
col-instance = Instance
col-ended-duration = Ended / duration
col-business-key-id = Business key / id
col-via = Via
col-flow = flow
col-target = target
col-condition = condition

## Inspector sections
section-identity = Identity
section-runtime = Runtime
section-in-this-instance = In this instance
section-execution = Execution
section-implementation = Implementation
section-user-task = User task
section-form = Form
section-event-definition = Event definition
section-outgoing-flows = Outgoing flows
section-connections = Connections
section-multi-instance = Multi-instance
section-input-output-mapping = Input / output mapping
section-listeners = Listeners
section-extension-properties = Extension properties
section-other-attributes = Other attributes
section-xml = XML
section-definition = Definition
section-instance = Instance
section-called-by = Called by
section-condition = Condition
section-now = Now

## Key/value row labels
kv-id = id
kv-name = name
kv-key = key
kv-type = type
kv-version = version
kv-documentation = documentation
kv-lane = lane
kv-pool = pool
kv-parent = parent
kv-attached-to = attached to
kv-interrupting = interrupting
kv-boundary-event = boundary event
kv-executed-history = executed (history)
kv-retry-time-cycle = retry time cycle
kv-called-element = called element
kv-script-format = script format
kv-script = script
kv-message = message
kv-message-ref = message ref
kv-signal = signal
kv-signal-ref = signal ref
kv-error = error
kv-error-code = error code
kv-code-variable = code variable
kv-message-variable = message variable
kv-escalation = escalation
kv-escalation-code = escalation code
kv-condition = condition
kv-language = language
kv-variable-name = variable name
kv-variable-events = variable events
kv-link = link
kv-compensates = compensates
kv-wait-for-completion = wait for completion
kv-default-flow = default flow
kv-from = from
kv-to = to
kv-mode = mode
kv-collection = collection
kv-element-variable = element variable
kv-cardinality = cardinality
kv-completion-condition = completion condition
kv-resource = resource
kv-deployment = deployment
kv-history-ttl = history TTL
kv-description = description
kv-business-key = business key
kv-definition = definition
kv-definition-id = definition id
kv-state = state
kv-started = started
kv-ended = ended
kv-duration = duration
kv-running-for = running for
kv-started-by = started by
kv-start-activity = start activity
kv-delete-reason = delete reason
kv-super-instance = super instance
kv-root-instance = root instance
kv-tenant = tenant
kv-history-removal = history removal
kv-task-id = task id
kv-due = due
kv-follow-up = follow-up
kv-priority = priority
kv-assignee = assignee
kv-annotation = annotation
kv-root-cause-incident = root cause incident
kv-in-instance = in instance
kv-job-id = job id
kv-retries = retries
kv-configuration = configuration
kv-exception = exception
kv-topic = topic
kv-worker = worker
kv-lock-expires = lock expires
kv-scope = scope
kv-job-retries = job retries
kv-job-error = job error
kv-terminate = terminate
kv-cancel = cancel
kv-expression = expression
kv-default = default

## BPMN element kinds
bpmn-task = Task
bpmn-user-task = User task
bpmn-service-task = Service task
bpmn-script-task = Script task
bpmn-business-rule-task = Business rule task
bpmn-send-task = Send task
bpmn-receive-task = Receive task
bpmn-manual-task = Manual task
bpmn-call-activity = Call activity
bpmn-subprocess = Subprocess
bpmn-ad-hoc-subprocess = Ad-hoc subprocess
bpmn-transaction = Transaction
bpmn-start-event = Start event
bpmn-end-event = End event
bpmn-intermediate-catch-event = Intermediate catch event
bpmn-intermediate-throw-event = Intermediate throw event
bpmn-boundary-event = Boundary event
bpmn-exclusive-gateway = Exclusive gateway
bpmn-parallel-gateway = Parallel gateway
bpmn-inclusive-gateway = Inclusive gateway
bpmn-event-based-gateway = Event-based gateway
bpmn-complex-gateway = Complex gateway
bpmn-pool = Pool
bpmn-lane = Lane
bpmn-data-object = Data object
bpmn-data-store = Data store
bpmn-data-input = Data input
bpmn-data-output = Data output
bpmn-text-annotation = Text annotation
bpmn-group = Group
bpmn-event-timer = Timer
bpmn-event-message = Message
bpmn-event-signal = Signal
bpmn-event-error = Error
bpmn-event-escalation = Escalation
bpmn-event-conditional = Conditional
bpmn-event-link = Link
bpmn-event-compensation = Compensation
bpmn-event-terminate = Terminate
bpmn-event-cancel = Cancel
bpmn-flow-sequence-flow = Sequence flow
bpmn-flow-message-flow = Message flow
bpmn-flow-association = Association
bpmn-flow-data-input-association = Data input association
bpmn-flow-data-output-association = Data output association

## Stragglers
palette-hint = Type a command, a definition key or name, an instance id or business key…   (> commands, # business key)
connect-url-hint = http://host:8080/engine-rest  (a Cockpit URL works too)
inc-resolve-title = Resolve incident
inc-resolve-body = Mark this incident as resolved? The underlying job or task is left untouched.
definitions-filter-hint = Filter by key or name

## Time
time-ago = { $d } ago
time-in = in { $d }
time-just-now = just now
dur-ms = { $ms } ms
dur-s = { $s } s
dur-m-s = { $m } m { $s } s
dur-h-m = { $h } h { $m } m
dur-d-h = { $d } d { $h } h
bpmn-event-generic = event

## Implementation / user-task attribute labels
kv-class = class
kv-delegate-expression = delegate expression
kv-result-variable = result variable
kv-task-priority = task priority
kv-decision-ref = decision ref
kv-decision-binding = decision binding
kv-decision-version = decision version
kv-decision-tenant = decision tenant
kv-map-decision-result = map decision result
kv-called-binding = called binding
kv-called-version = called version
kv-called-version-tag = called version tag
kv-called-tenant = called tenant
kv-variable-mapping-class = variable mapping class
kv-variable-mapping-delegate = variable mapping delegate
kv-candidate-users = candidate users
kv-candidate-groups = candidate groups
kv-due-date = due date
kv-form-key = form key
kv-form-ref = form ref
kv-form-binding = form binding
kv-form-version = form version

## Documentation blocks
doc-show-all = Show all ({ $n } more lines)
doc-show-less = Show less
kv-executable = executable
section-process = Process

## Opening files, document sources
cmd-open-file = Open BPMN file…
cmd-view-xml = View XML
cmd-compare = Compare BPMN…
open-file-title = Open a BPMN file
file-needs-engine = Needs a connected engine
source-offline = Not connected to an engine
source-no-data-dir = No data directory to export the definition to
settings-xml-editor = External XML editor
settings-xml-editor-hint = Command for “Open XML in editor”, e.g. code -g {"{"}file{"}"}:{"{"}line{"}"}:{"{"}col{"}"} or kate -l {"{"}line{"}"} -c {"{"}col{"}"} {"{"}file{"}"}. Empty hides the item.

## XML view
ctx-view-xml = View XML
ctx-open-xml-with = Open XML with…
ctx-open-xml-editor = Open XML in editor
ctx-copy-location = Copy path:line:col
ctx-copy-location-tip = The file and position of this element, as editors such as VS Code accept it (code -g path:line:col). Engine definitions are saved to the data directory first.
xml-find = Find
xml-find-hint = Find in XML  (Enter next, Shift+Enter previous)
xml-fold = { $n ->
    [one] { $n } unchanged line
   *[other] { $n } unchanged lines
}
xml-identical = identical
xml-whole-file = Whole file
xml-whole-file-tip = Show unchanged lines instead of folding them
xml-prev-change-tip = Previous change (Shift+N)
xml-next-change-tip = Next change (N)
xml-clear-highlight-tip = Remove the highlight (Esc)
xml-show-diagram = Show diagram
xml-show-diagram-tip = Switch to the diagram and select the highlighted element
xml-open-with = Open with…
xml-open-with-tip = Choose a program in the system's application chooser
xml-open-editor = Open in editor
xml-load-failed = Cannot load the document: { $error }
xml-export-failed = Cannot save the XML to disk
xml-editor-failed = Cannot start the editor
xml-open-with-failed = The application chooser failed
xml-not-loaded = The document is not loaded yet
xml-ctx-show-in-diagram = Show in diagram
xml-ctx-highlight-element = Highlight this element
xml-ctx-copy-line = Copy line
hint-xml = Ctrl+F find · F3 next match · Esc remove highlight · right-click a line: show in diagram, copy path:line:col
hint-xml-diff = N / Shift+N next / previous change · Ctrl+F find · Esc remove highlight · click ⋯ to unfold

## Diff view
diff-before = Before
diff-after = After
diff-choose = choose…
diff-swap-tip = Swap before and after
diff-mode-unified = Unified
diff-mode-before = Before
diff-mode-after = After
diff-mode-side-by-side = Side by side
diff-added = +{ $n } added
diff-removed = −{ $n } removed
diff-changed = ~{ $n } changed
diff-moved = ↔{ $n } moved
diff-filter-tip = Show or hide this kind of change
diff-no-changes = No differences
diff-all-filtered = Every change is hidden by the filter
diff-dim-unchanged = Dim unchanged
diff-xml = XML diff
diff-next-tip = Next change (N)
diff-prev-tip = Previous change (Shift+N)
diff-pick-file = Local file…
diff-pick-file-title = Choose a BPMN file to compare
diff-path-hint = or type a path
diff-use-path = Use
diff-no-such-file = No such file
diff-recent = Open and recent documents
diff-engine-definitions = Engine definitions
diff-filter-hint = filter by key or name
diff-setup-title = Compare two BPMN diagrams
diff-setup-body = Pick the version before and after: local files or definitions deployed on the engine.
diff-status-added = Added
diff-status-removed = Removed
diff-status-changed = Changed
diff-status-moved = Moved (layout only)
diff-also-moved = Also moved
diff-only-layout = Only position or size changed.
diff-id-changed = id was { $old }
diff-more-changes = …and { $n } more
section-document = Document
section-changes = Changes
dock-changes = Changes
hint-diff = N / Shift+N next / previous change · click a change to go there · Ctrl+U XML diff · right-click: XML of an element
