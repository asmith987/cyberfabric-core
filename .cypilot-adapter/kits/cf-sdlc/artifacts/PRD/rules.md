# PRD Rules (CyberFabric)

## Required References (ALWAYS)

ALWAYS open and follow `../../rules.md`

ALWAYS open and use `docs/spec-templates/cf-sdlc/PRD/template.md` as a template

ALWAYS open and follow `docs/checklists/PRD.md` as a quality checklist

## Required Headings (from constraints.json)

All headings support optional numbering prefix (e.g., `## 1. Overview`).

| # | Heading | Level | Required |
|---|---------|-------|----------|
| 1 | `## Overview` | H2 | yes |
| 2 | `### Purpose` | H3 | yes |
| 3 | `### Background / Problem Statement` | H3 | yes |
| 4 | `### Goals (Business Outcomes)` | H3 | yes |
| 5 | `### Glossary` | H3 | yes |
| 6 | `## Actors` | H2 | yes |
| 7 | `### Human Actors` | H3 | yes |
| 8 | `### System Actors` | H3 | yes |
| 9 | `## Operational Concept & Environment` | H2 | yes |
| 10 | `## Scope` | H2 | yes |
| 11 | `### In Scope` | H3 | yes |
| 12 | `### Out of Scope` | H3 | yes |
| 13 | `## Functional Requirements` | H2 | yes |
| 14 | `## Non-Functional Requirements` | H2 | yes |
| 15 | `### NFR Exclusions` | H3 | no |
| 16 | `## Public Library Interfaces` | H2 | yes |
| 17 | `### Public API Surface` | H3 | no |
| 18 | `### External Integration Contracts` | H3 | no |
| 19 | `## Use Cases` | H2 | yes |
| 20 | `## Acceptance Criteria` | H2 | yes |
| 21 | `## Dependencies` | H2 | yes |
| 22 | `## Assumptions` | H2 | yes |
| 23 | `## Risks` | H2 | yes |
| 24 | `## Open Questions` | H2 | no |
| 25 | `## Traceability` | H2 | no |

## ID Kinds (from constraints.json)

| Kind | Template | Required | Task | Priority | Heading | to_code |
|------|----------|----------|------|----------|---------|---------|
| `actor` | `cpt-{system}-actor-{slug}` | no | prohibited | prohibited | `### Human Actors` / `### System Actors` | no |
| `fr` | `cpt-{system}-fr-{slug}` | **yes** | **required** | **required** | `## Functional Requirements` | no |
| `nfr` | `cpt-{system}-nfr-{slug}` | **yes** | **required** | **required** | `## Non-Functional Requirements` | no |
| `usecase` | `cpt-{system}-usecase-{slug}` | no | allowed | allowed | `## Use Cases` | no |
| `interface` | `cpt-{system}-interface-{slug}` | no | allowed | allowed | `### Public API Surface` | no |
| `contract` | `cpt-{system}-contract-{slug}` | no | allowed | allowed | `### External Integration Contracts` | no |

**Definition forms**:
- `actor`: plain `**ID**: \`cpt-...\`` (task/priority prohibited)
- `fr`, `nfr`: checkbox `- [ ] \`p1\` - **ID**: \`cpt-...\`` (task/priority required)
- `usecase`, `interface`, `contract`: either form (task/priority allowed)

## Cross-Artifact References

| PRD Kind | Target Artifact | Coverage | Target Heading |
|----------|-----------------|----------|----------------|
| `fr` | DESIGN | **required** | `### Architecture Drivers` |
| `fr` | DECOMPOSITION | optional | feature entries |
| `fr` | FEATURE | optional | `### Purpose` |
| `nfr` | DESIGN | **required** | `### Architecture Drivers` |
| `nfr` | DECOMPOSITION | optional | feature entries |
| `nfr` | FEATURE | optional | `### Purpose` |
| `usecase` | DESIGN | optional | `### Interactions & Sequences` |
| `usecase` | FEATURE | optional | any |
| `interface` | DESIGN | optional | `### API Contracts` |
| `contract` | DESIGN | optional | `### API Contracts` |

**Critical**: every `fr` and `nfr` MUST be referenced somewhere in DESIGN (under Architecture Drivers) for validation to pass.

PRD MUST NOT backtick-reference ADR IDs (coverage: **prohibited**).

## Generation Checklist

- [ ] Populate all required headings (see table above); no TODO/TBD/FIXME placeholders.
- [ ] Define concrete actors (human + system) under correct headings; `actor` IDs use plain form (no checkbox).
- [ ] Write `fr`/`nfr` with checkbox definition form; assign priorities `p1`-`p9`.
- [ ] Write measurable success criteria (baseline + target + timeframe where possible).
- [ ] Define FRs/NFRs as WHAT, not HOW; include actor references where relevant.
- [ ] Ensure every listed capability is backed by at least one FR and at least one use case.
- [ ] Keep implementation details (routes/DB schemas) out of PRD; defer to DESIGN/FEATURE.
- [ ] Do NOT backtick-reference ADR IDs from PRD.

## Validation Checklist

- [ ] `python3 {cypilot_path}/skills/cypilot/scripts/cypilot.py validate --artifact <path>`
- [ ] `make cypilot-validate`
- [ ] Review against `docs/checklists/PRD.md`.

## Review Checklist

- [ ] No implementation details — defer to DESIGN/FEATURE
- [ ] Challenge vague, overlapping, or untestable items
- [ ] All `fr`/`nfr` have checkbox form with priority
- [ ] `actor` IDs use plain form (no checkbox)
- [ ] No backtick references to ADR/DESIGN IDs
- [ ] Run `docs/checklists/PRD.md` - "PR Review Focus (Requirements)"
