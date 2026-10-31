# Spec Delta

## ADDED Requirements

### Requirement: Mouse input queued before an overlay
A click, tab drop or right-click made while no menu or rename field was drawn, which finds one open at its turn, SHALL close it without running it and then act as usual. Wheel turns, drags and other mouse input for a program, and border-drag positions, that find a menu or rename field open at their turn SHALL be dropped. While a menu or rename field is drawn, mouse input SHALL go to it in input order.

#### Scenario: Click queued behind a rename key
- **WHEN** 500 ms of delay is added each way, and the user presses `alt-n`, then `alt-p` and `r`, and at once clicks another pane
- **THEN** the field opens, then closes without sending anything, and the clicked pane is selected

#### Scenario: Wheel queued behind a rename key
- **WHEN** 500 ms of delay is added each way, the selected pane runs a program that tracks the mouse, and the user presses `alt-n`, then `alt-p` and `r`, and at once turns the wheel over that pane
- **THEN** the field stays open and the program receives no wheel input
