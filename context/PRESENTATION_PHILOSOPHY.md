# Presentation Philosophy

## Scope / Purpose

- This document records the current design and development philosophy for Aurix presentation surfaces, based on actual project discussions, so that GUI and future TUI work do not drift into conflicting styles.

## Current Implemented System

- The current GUI follows a visual-first direction where charts and comparative surfaces are intended to carry more explanatory weight than large blocks of numeric text.
- The current GUI styling is centralised in shared CSS files rather than scattered inline or utility-class-heavy component styling.
- The current GUI is intentionally dark and modern, but the design direction has been corrected away from overly bubbly panels, heavy gradients, roadmap copy, and decorative noise.

## Implemented Outputs / Artifacts

- The existing GUI uses compact supporting text, restrained panel styling, pastel accents, and a chart-first hero surface.
- The existing frontend styling is managed through `src/styles/theme.css` and `src/styles/dashboard.css`.

## In Progress / Partially Implemented

- The chart system is still being refined to become both readable and impressive instead of merely decorative.
- The GUI now points in the right direction stylistically, but the chart interaction and runtime clarity still need work.
- The future TUI philosophy has been discussed, but nothing has been implemented for it yet.

## Planned / Missing / To Be Changed

- Keep the GUI visually calm: thin accents, plain or low-gradient surfaces, smaller supporting text, and charts as the dominant explanatory surface.
- Avoid rendering development roadmap content, milestone labels, or internal implementation notes inside the product UI.
- Keep the TUI visually inspired by strong monitoring tools such as `gotop` or `btop`, meaning dense but legible information, graph-led scanning, and fast at-a-glance state reading.
- Maintain similarity between GUI and TUI at the information-architecture level even if the rendering medium differs.

## Notes / Design Considerations

- “Modern” in Aurix should mean clean, restrained, and precise rather than bubbly, glossy, or overloaded with gradients.
- Numbers should support interpretation, but the screen should primarily answer questions visually.
- A chart should only be prominent if it is analytically meaningful, not just because charts look impressive.
- Shared presentation rules should remain centralised so theming and later cross-tab polish can be applied globally.

## Discarded / Obsolete / No Longer Relevant

- The earlier bubbly, roadmap-flavoured GUI direction is obsolete.
- The earlier default Tauri starter look is obsolete.
