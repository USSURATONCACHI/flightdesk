---
name: flightdeck
description: Operate a Camunda 7 engine and the Flightdeck desktop cockpit through its built-in MCP server (tools prefixed flightdeck/ mcp__flightdeck__*). Use when the user asks about process definitions, process instances, tokens, variables, incidents, jobs, user tasks, external tasks, token modification, batches, or wants the Flightdeck UI driven (navigate, select elements, zoom, screenshots).
---

# Flightdeck (Camunda 7 cockpit) via MCP

The Flightdeck app must be running with its MCP server enabled (top bar pill "MCP :7788", or
Settings → MCP server, or start it with `flightdeck --mcp`). The server listens on
`http://127.0.0.1:7788/mcp` (Streamable HTTP). If tools are missing from this session, the
project `.mcp.json` registers it; approve it or run
`claude mcp add --transport http flightdeck http://127.0.0.1:7788/mcp` and restart the session.

## Orientation
1. `engine_status` — connected? engine URL/version.
2. `ui_state` — what the user is looking at (place, selection, dock tab, zoom, pending confirm).
3. Read before you write: `get_instance`, `get_variables`, `get_activity_history`,
   `get_element`, `list_incidents`, `get_error_details`.

## Reading a process
- `list_definitions` → `get_definition` (activity statistics = tokens and incidents per element).
- `list_elements` (structure, lanes, in/out), `get_element` (every attribute, Camunda extensions,
  conditions, XML), `find_elements` (text search, same as Ctrl+F).
- Instance search language for `search_instances` / `ui_navigate search:`:
  `def:key ver:3 bk:ABC* act:Task_x exec:Task_y var.amount>1000 var.name="x y" state:active|suspended|running|finished|completed|terminated|any inc:yes|failedJob|~"msg" started>-7d finished<2026-10-01 by:user super:uuid`.

## Acting
- Variables: `set_variable` (type inferred unless given), `delete_variable` (confirm).
- Incidents/jobs: `retry_job`, `execute_job`, `set_job_duedate`, `retry_external_task`,
  `unlock_external_task`, `resolve_incident`, `annotate_incident`.
- Tasks: `set_task_assignee`, `complete_task` (confirm).
- Instances: `start_instance`, `suspend_instance`, `delete_instance` (confirm),
  `modify_instance` (token move/cancel/start; confirm), `restart_instance` (confirm),
  `correlate_message`, `batch` (bulk; confirm).
- Destructive tools need `confirm: true`; tell the user what you are about to do first unless they
  already asked explicitly.

## Driving the UI (show the user what you found)
- `ui_navigate` (`def:<key>`, `instance:<id>`, `search:<query>`, `incidents`, `batches`),
  `ui_select` + `ui_fit`, `ui_find`, `ui_dock` (Variables, Incidents, Timeline, Modify…),
  `ui_filter`, `ui_zoom`, `ui_pan`, `ui_tabs`, `ui_command` / `ui_commands`, `ui_key`, `ui_type`,
  `ui_confirm` (answer the app's own confirmation dialog), `ui_screenshot` (see the screen).
- After navigation give the UI a moment, then `ui_state` or `ui_screenshot` to verify.
