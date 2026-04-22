# No Inline Rationale

## Current Understanding

The Aurix codebase has **no** `WHY`/`NOTE`/`HACK`/`IMPORTANT`/`TODO`/`SAFETY`/`FIXME`/`XXX` annotations in any source file. A repository-wide grep returns zero matches outside `context/`. Git commit bodies are also thin — 5 of 6 commits on `master` are subject-only (`project init`, `worked on milestone 1`, `ticked off readme`, `added insights`, `restructured context folder`); the only commit with a body is the initial Tauri scaffold, which describes what was bootstrapped rather than durable rationale.

All design rationale for the project lives in `context/` — primarily in the system files' Durable Notes sections and in these notes files.

## Guiding Principles

- **Put rationale in `context/`, not in code.** When a design decision is non-obvious enough that a future reader would ask "why is this done this way?", capture it in the owning system file's Durable Notes section or in a topical note here. Do not leave it as an inline `// WHY …` comment; the project's convention is that those comments do not exist, and adding them creates two canonical homes for the same knowledge.
- **Write comments only when the code alone is unclear.** Per `CLAUDE.md` §Engineering Standards, inline comments are for intent that the code does not make obvious. Rationale is a different category and belongs in context docs.
- **Commit messages are allowed to be thin, but not rationale-carrying decisions.** The project's cadence has been short subjects. That is fine for ticking off milestones, but non-trivial design choices (a new abstraction, a reversal of a prior approach, a constraint accepted) should either land in a commit body or — better — in a context note as the commit is made.
- **Future upkeep passes should re-check this assumption.** If source annotations start appearing (especially `TODO` or `FIXME`), that is a drift signal: either the convention is changing intentionally (update this note) or the comments are bypassing the rationale-capture path (surface them to a system file).

## Rationale

This note exists because the `upkeep-context` skill's rationale-capture obligation specifically requires grep-ing source annotations and inspecting `git log --format=fuller` bodies. Stating the absence explicitly — rather than silently finding nothing — makes the convention visible to future sessions and prevents future upkeep from re-deriving "huh, there are no comments" as if it were a gap rather than a choice.
