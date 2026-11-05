# Spec Delta

## MODIFIED Requirements

### Requirement: Tab sidebar presentation
The sidebar's tabs section SHALL show a plain `tabs` title above one entry per visible tab in tree order, indented by depth and using derived labels. Tabs with children SHALL have fold markers. It SHALL highlight the selected tab or the selected pane's tab, and no entry when nothing is selected. A zoomed tab's entry SHALL carry a zoom marker; while the sidebar is hidden, no zoom marker is shown. A tab entry SHALL end its first line with its rollup icon as agent-attention's Tab rollup describes.

#### Scenario: Nested rows
- **WHEN** a client opens a three-level tab tree and folds an intermediate row
- **THEN** the sidebar retains that row with a folded marker and hides its descendants without hiding their programs

#### Scenario: Zoom marker
- **WHEN** the selected tab is zoomed
- **THEN** its sidebar row carries a zoom marker, without requiring a pane border

#### Scenario: Empty selection beside sidebar
- **WHEN** nothing is selected or an empty tab is selected
- **THEN** the existing corresponding empty-state hint appears in the pane area beside the sidebar

#### Scenario: Rollup icon
- **WHEN** a pane in a tab's child tab is `working`
- **THEN** both tabs' rows end with the working icon

### Requirement: Tab drop line
While a row is dragged, the drop line SHALL be an underline on the cells of the line above the gap, from the target depth's indent to the divider; the gap above the first entry SHALL use the blank row under the heading. Gaps SHALL fall only between tab entries, so the line above a gap is the last line of an entry.

#### Scenario: Drop line
- **WHEN** a user drags a row over the gap below a top-level tab with the pointer one indent further right
- **THEN** the row above the gap is underlined from that indent to the divider, and no row moves until release

#### Scenario: Drop onto a second line
- **WHEN** entries have two lines and a user drags a row over the second line of tab `api`
- **THEN** the gap is the one above `api`'s entry, and the last line of the entry above `api` is underlined

## ADDED Requirements

### Requirement: Tab entry lines
Each tab's entry SHALL be drawn from the lines of `[client.tabs] rows` (see config-file's Tab settings). `name` SHALL be the tab's derived label with its zoom marker. A tag item SHALL be the value described by Tab entry tags. An item with no value or an empty value SHALL be left out with its separator, and a line left with nothing SHALL be left out. An entry SHALL keep at least one line, blank when nothing has a value.

#### Scenario: Two-line entry
- **WHEN** `rows = [["name"], [{ tag = "git.branch" }]]` and tab `web` has `git.branch` `main`
- **THEN** its entry shows `web` with `main` on the line below

#### Scenario: Missing tag
- **WHEN** with those rows, tab `notes` and its first pane have no `git.branch`
- **THEN** its entry is one line, `notes`

#### Scenario: Default rows
- **WHEN** the config has no `[client.tabs]`
- **THEN** every entry is one line holding the derived label and zoom marker, as before this change

### Requirement: Fitting tab lines
Items on a tab entry's line SHALL be joined with ` · `. A line too wide for the sidebar SHALL share its width as agent-attention's Fitting agent lines describes, each item losing its end; the zoom marker SHALL keep its width.

#### Scenario: Long branch under a nested tab
- **WHEN** a 26-column sidebar shows tab `deep` two levels down with `git.branch` `feature/very-long-branch-name` and `git.ahead` `2` on one line
- **THEN** that line reads `feature/ver… · 2`

### Requirement: Tab entry tags
In tab entries, a tag item SHALL show the tab's own tag of that name, else its first own pane's tag of that name, the pane that derived labels use. This fallback SHALL be a display rule only: `ship tab get`, `ship tab list` and `ship watch` SHALL show only the tab's own tags.

#### Scenario: Fallback to the first pane
- **WHEN** tab `web` has no `git.branch` and its first pane has `git.branch` `main`
- **THEN** the entry shows `main`, and `ship tab get` for `web` has no `tags` field

#### Scenario: The tab's own tag wins
- **WHEN** tab `web` has `git.branch` `release` and its first pane has `main`
- **THEN** the entry shows `release`

### Requirement: Tab entry lines and the mouse
Every line of a tab's entry SHALL act as that tab's row for clicks, drags and the wheel. Only the first line SHALL carry the fold marker and the rollup icon. The selected tab's highlight SHALL cover every line of its entry, and moving the selection SHALL keep its whole entry in view.

#### Scenario: Click a second line
- **WHEN** a user clicks the second line of tab `api`'s entry
- **THEN** `api` is selected, as a click on its first line would

#### Scenario: Marker only on the first line
- **WHEN** a tab with children has a two-line entry and a user clicks the start of its second line
- **THEN** the tab is selected and its fold doesn't change

### Requirement: Tag colors in the sidebar
A tag's `fg` and `bg` SHALL apply to its value's cells only. Separators and the rest of the line SHALL keep the line's style, and on the selected entry the tag's colors SHALL show over the highlight.

#### Scenario: Colored tag on the selected tab
- **WHEN** the selected tab shows a tag with `fg` `white` and `bg` `red`
- **THEN** that value's cells are white on red, and the rest of the entry keeps the selection highlight
