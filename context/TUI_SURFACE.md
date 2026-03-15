# TUI Surface

## Scope / Purpose

- This document tracks the planned terminal interface for Aurix and records the intended relationship between the TUI and the shared analytics core.

## Current Implemented System

- No TUI exists in the repository at present.
- The backend structure is intentionally being shaped so that a future TUI can consume the same market and analytics outputs as the GUI.

## Implemented Outputs / Artifacts

- There is no TUI artifact yet beyond the architectural intent captured in documentation and the existing shared backend boundaries.

## In Progress / Partially Implemented

- The current backend separation between config, RPC, DEX adapters, and market models is compatible with a future TUI, but no terminal entrypoint or render layer has been started.

## Planned / Missing / To Be Changed

- Add a TUI entrypoint that reuses the shared analytics core rather than reimplementing venue reads.
- Design the TUI around graph-led monitoring and dense at-a-glance scanning in the spirit of `gotop` or `btop`.
- Keep the TUI lighter-weight than the GUI while preserving the same underlying market, spread, and opportunity concepts.
- Preserve cross-surface consistency so the GUI and TUI answer the same core questions even if their layouts differ.

## Notes / Design Considerations

- The TUI should not become a second product with its own analytics logic; it should be another view over the same core systems.
- The TUI should be particularly useful for Linux and terminal-centric usage, which is why it remains an active planned surface rather than a discarded idea.
- Graphs and bars in the TUI should be information-dense and operational rather than ornamental.

## Discarded / Obsolete / No Longer Relevant

- The TUI idea is not discarded.
