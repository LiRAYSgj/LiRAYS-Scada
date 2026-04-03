# shadcn-svelte Refactor Audit

Date: 2026-04-03
Scope: `frontend/src` app UI
Reference docs cache:
- `doc/shadcn-svelte/components/*.md` (59 component docs)
- `doc/shadcn-svelte/llms.txt`

## Already aligned (keep)

These areas are already based on shadcn-svelte primitives and should mostly be kept:
- `frontend/src/lib/components/ui/button/*`
- `frontend/src/lib/components/ui/dialog/*`
- `frontend/src/lib/components/ui/field/*`
- `frontend/src/lib/components/ui/form/*`
- `frontend/src/lib/components/ui/input/*`
- `frontend/src/lib/components/ui/label/*`
- `frontend/src/lib/components/ui/number-field/*`
- `frontend/src/lib/components/ui/select/*`
- `frontend/src/lib/components/ui/separator/*`
- `frontend/src/lib/components/ui/tags-input/*`
- `frontend/src/lib/components/ui/textarea/*`

## High-impact refactor opportunities

1. `frontend/src/lib/components/Snackbar/Snackbar.svelte`
- Current: custom popover/snackbar implementation.
- Refactor to: `sonner`.
- Why: better accessibility/stacking/transitions and removes custom portal + timer plumbing.
- Docs: `doc/shadcn-svelte/components/sonner.md`

2. `frontend/src/lib/features/tree/components/ContextMenu.svelte`
- Current: custom layered context menu + submenu logic.
- Refactor to: `context-menu` (or `dropdown-menu` where right-click is not required).
- Why: remove custom submenu positioning/loading layer complexity and standardize keyboard behavior.
- Docs: `doc/shadcn-svelte/components/context-menu.md`, `doc/shadcn-svelte/components/dropdown-menu.md`

3. `frontend/src/lib/features/tree/components/OverlaySurface.svelte` + `TagMetadataTooltip.svelte`
- Current: custom fixed overlay primitive used for tooltip-like panel.
- Refactor to: `tooltip` for short metadata, `hover-card`/`popover` for richer metadata.
- Why: simplifies top-layer positioning/focus concerns and keeps behavior consistent with preset.
- Docs: `doc/shadcn-svelte/components/tooltip.md`, `doc/shadcn-svelte/components/hover-card.md`, `doc/shadcn-svelte/components/popover.md`

4. `frontend/src/lib/features/tree/components/TreeRow.svelte`
- Current: custom checkbox glyph and state rendering.
- Refactor to: `checkbox`.
- Why: consistent checked/indeterminate visuals and semantics with fewer custom SVG branches.
- Docs: `doc/shadcn-svelte/components/checkbox.md`

5. `frontend/src/lib/features/workspace/components/PageToolbar.svelte`
- Current: custom account display and action grouping.
- Refactor to: `avatar`, `button-group`, optional `dropdown-menu` for account actions.
- Why: closer to preset component language, less custom account-card styling.
- Docs: `doc/shadcn-svelte/components/avatar.md`, `doc/shadcn-svelte/components/button-group.md`, `doc/shadcn-svelte/components/dropdown-menu.md`

## Medium-impact refactor opportunities

6. `frontend/src/lib/components/Button/Button.svelte`
- Current: custom compatibility wrapper over `ui/button` with extra variants.
- Refactor to: direct `ui/button` usage + small project-specific variant extension in one place.
- Why: reduces duplication and variant drift; easier global style control.
- Docs: `doc/shadcn-svelte/components/button.md`

7. `frontend/src/lib/features/namespace-builder/components/NamespaceBuilderHeader.svelte`
- Current: custom tab-like buttons for `Visual Tree` / `YAML`.
- Refactor to: `tabs`.
- Why: built-in semantics and selected-state behavior.
- Docs: `doc/shadcn-svelte/components/tabs.md`

8. `frontend/src/lib/features/namespace-builder/components/NamespaceBuilder.svelte`
- Current: custom scroll containers and empty/error messaging blocks.
- Refactor to: `scroll-area` for tree viewport container, `alert` for parse/init errors.
- Why: consistent scrolling affordances and message styling across light/dark.
- Docs: `doc/shadcn-svelte/components/scroll-area.md`, `doc/shadcn-svelte/components/alert.md`

9. `frontend/src/lib/features/namespace-builder/components/NamespaceBuilderTreeRow.svelte`
- Current: custom “Folder” pill and ad-hoc inline chips.
- Refactor to: `badge` for folder/type status tags; optional `tooltip` for icon actions.
- Why: visual consistency with preset chips/badges and clearer action affordances.
- Docs: `doc/shadcn-svelte/components/badge.md`, `doc/shadcn-svelte/components/tooltip.md`

10. `frontend/src/routes/+page.svelte` (Inspector panel)
- Current: ad-hoc bordered cards and custom tag chips.
- Refactor to: `card` for inspector sections, `badge` for binding chips, `separator` between sections.
- Why: improve consistency with preset’s container language.
- Docs: `doc/shadcn-svelte/components/card.md`, `doc/shadcn-svelte/components/badge.md`, `doc/shadcn-svelte/components/separator.md`

## Low-impact / optional

11. `frontend/src/routes/+page.svelte` remove confirmation modals
- Current: uses `dialog` with destructive actions.
- Optional refactor to: `alert-dialog` for destructive confirmations.
- Why: semantic intent for irreversible actions.
- Docs: `doc/shadcn-svelte/components/alert-dialog.md`

12. Tree/namespace table-like headers and rows
- Current: custom CSS grid rows for virtualization and drag/drop.
- Optional refactor to: `table` for static parts only.
- Note: full `table` migration may conflict with virtualized row rendering and DnD hit-testing.
- Docs: `doc/shadcn-svelte/components/table.md`

## Suggested implementation order

1. Migrate snackbar to `sonner`.
2. Replace custom context menu stack with `context-menu`.
3. Replace custom tree checkbox with `checkbox`.
4. Migrate toolbar account block to `avatar` + grouped buttons.
5. Convert namespace header to `tabs`; add `scroll-area` and `badge` where low risk.
6. Revisit wrapper `Button` simplification after above migrations.

## Notes

- Virtualized trees (`VariableTree`, `NamespaceBuilder`) should keep current virtualization logic; only swap row sub-controls/pills to shadcn components.
- Keep `NumberField` and `TagsInput` as-is since they are already aligned with your custom field requirements.
