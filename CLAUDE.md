# Trek — Vision & Purpose

Trek is a **terminal-first visual file browser** designed to live inside [cmux](https://github.com/bradleyfay/cmux) as a persistent project panel. It gives you the navigation and browsing experience of VSCode's file explorer, but entirely in the terminal — no GUI, no Electron, no leaving your workflow.

---

## What Trek Is

Trek is a **project browser, not a text editor.**

The mental model is VSCode's sidebar file tree, elevated to full-pane status and wired into the terminal ecosystem. You open trek in a cmux pane, navigate your project, and use it to launch the right tool for each file type — not to be that tool itself.

### Primary jobs:

- **Explore** a project's structure at a glance (three-pane: parent / current / preview)
- **Navigate** quickly to any file or directory without typing full paths
- **Open** files in the appropriate viewer or editor for their type
- **Browse** file contents in the preview pane before deciding what to do
- **Search** inside files across a project (ripgrep-powered)
- **Make minor edits** in-place when it's not worth switching context

Trek is the thing you use to *find* and *route* files, not the final destination for working with them.

---

## cmux Integration — The Primary Context

Trek is designed to run as a **persistent pane within cmux**. This shapes every design decision:

### File routing by type

Trek doesn't try to do everything itself. It hands files off to the right cmux-aware tool:

| File type | Opens in |
|---|---|
| Markdown (`.md`) | cmux markdown viewer |
| HTML (`.html`, `.htm`) | System browser |
| Images | System viewer or cmux image preview |
| Code / text | Editor pane (configurable — defaults to `$EDITOR`) |
| PDFs | System viewer |
| Directories | Navigate in-place (trek stays the browser) |

The guiding rule: **trek opens the right tool, not a worse version of it.**

### What trek handles directly

- Reading and previewing text files in the right pane
- In-place edits for small changes (renaming, quick fixes, config tweaks)
- File operations: copy, move, delete, mkdir
- Git status overlays (modified, staged, untracked, deleted)
- Bulk rename with regex and live preview
- Content search across the project

---

## What Trek Is Not

Knowing what trek doesn't do is as important as knowing what it does.

**Trek is not a text editor.** There is no intent to build a full editing environment. The editing surface exists for quick, minor changes — not for writing code. If you find yourself wanting syntax highlighting, LSP hints, or multi-file editing, that's a signal to open your actual editor.

**Trek is not a vim clone.** Vim keybindings exist where they overlap with universal conventions (`j`/`k` for up/down, `g`/`G` for top/bottom), but trek does not require vim fluency. Every keybinding must have a visible hint — either always-on in the UI or immediately accessible via the command palette. Muscle memory is optional; discoverability is required.

**Trek is not a shell.** It does not replace your terminal. Shell integration (`trek --install-shell`) provides a quality-of-life bridge (the `m` command that `cd`s on exit), but trek's job is navigation and browsing, not command execution.

**Trek is not a fuzzy launcher.** File search in trek (`/` and `Ctrl+F`) is scoped to the current project. It is not a global file finder or application launcher.

---

## Design Principles

### Terminal-first, not editor-first

Trek is optimized for people who live in the terminal but don't necessarily live in vim. The experience should feel as comfortable to someone who uses nano or a GUI editor as it does to someone fluent in modal editing.

### Discoverability over memorability

Every action should be reachable without prior knowledge:
- A **command palette** (`Ctrl+P` or `?`) lists all available actions with their keybindings
- Vim-style bindings are labeled and explained — never assumed
- Status bars and overlays surface context-relevant hints
- Mouse is a first-class input method alongside keyboard

### The right tool for the job

Trek's value is in routing. A good trek session might touch six different tools without the user consciously switching applications. Trek provides the connective tissue.

### Stay out of the way

Trek should take up exactly one cmux pane and nothing more. It should not spawn full-screen overlays that displace your layout. Modals, search, and rename previews all operate within trek's pane boundaries.

---

## Keybinding Philosophy

Trek uses keyboard shortcuts, but they are **not a prerequisite**. The command palette and mouse support mean a user who has never seen trek before can be productive immediately.

For keyboard shortcuts that borrow from vim convention:

1. They must appear in the help overlay (`?`)
2. They must be listed in the command palette
3. Where possible, they should have an alternative that is more intuitive (e.g., arrow keys alongside `hjkl`)

New keybindings added to trek should follow this checklist:
- Does it conflict with an existing binding?
- Is it listed in the help overlay?
- Is it reachable from the command palette?
- Does it have a mouse equivalent where applicable?

---

## Command Palette

The command palette is the **primary discoverability surface** for trek. It should:

- Open with `Ctrl+P` (VSCode-familiar) or `?`
- List all available actions in the current context
- Show the keybinding for each action
- Support fuzzy filtering by action name
- Execute the selected action directly

The command palette is not a separate mode — it is an overlay that closes after one action. Think of it as "searchable help that also works."

---

## Scope Boundaries — Quick Reference

| In scope | Out of scope |
|---|---|
| File tree navigation | Full text editing |
| File preview (read-only) | Syntax highlighting in editor |
| Minor in-place edits | LSP / code intelligence |
| File operations (copy, move, delete, mkdir) | Terminal emulation |
| Bulk rename with regex | Plugin system |
| Content search (ripgrep) | Remote file systems |
| Git status overlays | Git operations beyond status |
| Opening files in cmux tools / browser | Managing cmux layout |
| Mouse-resizable panes | Window management |
| Command palette | Application launching |
| Shell integration (`cd` on exit) | Scripting / macros |

---

## Relationship to AGENTS.md

`AGENTS.md` covers how to build trek — workflow, testing, commits, releases.

This document covers **what to build and why**. When a feature request arrives, check it against this vision first. If a proposed feature belongs in an editor, a shell, or a separate tool, it does not belong in trek.
