# Spec Delta

## MODIFIED Requirements

### Requirement: Published screen styles
A row's `runs` SHALL give its cells' styles from the left, each run covering `cells` adjacent cells with one style made of `fg`, `bg`, `underline_color` and `attrs`. Cells past the last run SHALL have the default style, and a row SHALL NOT end with a default-style run. A run's default-valued fields SHALL be omitted.

#### Scenario: Colored underlines reach the client
- **WHEN** a pane's program writes text with a colored underline, as Neovim does for diagnostics
- **THEN** the screen's run for that text carries `underline_color`, and the client draws the underline in that color
