# DESIGN Rules (CyberFabric)

## Required References (ALWAYS)

ALWAYS open and follow `../../rules.md`

ALWAYS open and use `docs/spec-templates/cf-sdlc/DESIGN/template.md` as a template

ALWAYS open and follow `docs/checklists/DESIGN.md` as a quality checklist

## Required Headings (from constraints.json)

All headings support optional numbering prefix (e.g., `## 1. Architecture Overview`).

| # | Heading | Level | Required |
|---|---------|-------|----------|
| 1 | `## Architecture Overview` | H2 | yes |
| 2 | `### Architectural Vision` | H3 | yes |
| 3 | `### Architecture Drivers` | H3 | yes |
| 4 | `### Architecture Layers` | H3 | yes |
| 5 | `## Principles & Constraints` | H2 | yes |
| 6 | `### Design Principles` | H3 | yes |
| 7 | `### Constraints` | H3 | yes |
| 8 | `## Technical Architecture` | H2 | yes |
| 9 | `### Domain Model` | H3 | yes |
| 10 | `### Component Model` | H3 | yes |
| 11 | `### API Contracts` | H3 | yes |
| 12 | `### Internal Dependencies` | H3 | no |
| 13 | `### External Dependencies` | H3 | no |
| 14 | `### Interactions & Sequences` | H3 | yes |
| 15 | `### Database schemas & tables` | H3 | yes |
| 16 | `## Additional context` | H2 | no |
| 17 | `## Traceability` | H2 | no |

## ID Kinds (from constraints.json)

| Kind | Template | Required | Task | Priority | Heading | to_code |
|------|----------|----------|------|----------|---------|---------|
| `principle` | `cpt-{system}-principle-{slug}` | **yes** | allowed | allowed | `### Design Principles` | no |
| `constraint` | `cpt-{system}-constraint-{slug}` | **yes** | allowed | allowed | `### Constraints` | no |
| `component` | `cpt-{system}-component-{slug}` | **yes** | allowed | allowed | `### Component Model` | no |
| `seq` | `cpt-{system}-seq-{slug}` | **yes** | allowed | allowed | `### Interactions & Sequences` | no |
| `interface` | `cpt-{system}-interface-{slug}` | no | allowed | allowed | `### API Contracts` | no |
| `db` | `cpt-{system}-db-{slug}` | no | allowed | allowed | `### Database schemas & tables` | no |
| `dbtable` | `cpt-{system}-dbtable-{slug}` | no | allowed | allowed | `### Database schemas & tables` | no |
| `design` | `cpt-{system}-design-{slug}` | no | allowed | allowed | H1 title | no |
| `topology` | `cpt-{system}-topology-{slug}` | no | allowed | allowed | `## Technical Architecture` | no |
| `tech` | `cpt-{system}-tech-{slug}` | no | allowed | allowed | `### Architecture Layers` | no |

All DESIGN ID kinds have task/priority **allowed** (not required) — use either checkbox or plain `**ID**:` form.

## Cross-Artifact References

### Inbound (IDs defined elsewhere, referenced in DESIGN)

| Source | Kind | Coverage | Where in DESIGN |
|--------|------|----------|-----------------|
| PRD | `fr` | **required** | `### Architecture Drivers` |
| PRD | `nfr` | **required** | `### Architecture Drivers` |
| PRD | `usecase` | optional | `### Interactions & Sequences` |
| PRD | `interface` | optional | `### API Contracts` |
| PRD | `contract` | optional | `### API Contracts` |
| ADR | `adr` | **required** | `### Architecture Drivers` |

**Critical**: every PRD `fr`/`nfr` MUST appear as a backtick reference in DESIGN under `### Architecture Drivers`. Every ADR `adr` ID MUST be referenced in DESIGN.

### Outbound (DESIGN IDs referenced downstream)

| DESIGN Kind | Target Artifact | Coverage | Target Heading |
|-------------|-----------------|----------|----------------|
| `component` | DECOMPOSITION | **required** | feature entries |
| `principle` | DECOMPOSITION | optional | feature entries |
| `principle` | FEATURE | optional | `### Purpose` |
| `constraint` | DECOMPOSITION | optional | feature entries |
| `constraint` | FEATURE | optional | `## Definitions of Done` |
| `seq` | DECOMPOSITION | optional | feature entries |
| `db`/`dbtable` | DECOMPOSITION | optional | feature entries |
| `db`/`dbtable` | FEATURE | optional | `## Definitions of Done` |

**Critical**: every `component` MUST be covered by at least one DECOMPOSITION feature entry.

### ADR Inline Pattern

Principles and constraints that are backed by an ADR SHOULD include an inline ADR reference:

```markdown
#### {Principle Name}

- [ ] `p2` - **ID**: `cpt-{system}-principle-{slug}`

**ADRs**: `cpt-{system}-adr-{slug}`

{Description}
```

## Generation Checklist

- [ ] Include all required headings (see table above); no TODO/TBD/FIXME placeholders.
- [ ] Reference PRD `fr`/`nfr` IDs in `### Architecture Drivers` (required for traceability).
- [ ] Reference ADR IDs in `### Architecture Drivers` and inline on related principles/constraints.
- [ ] Define `principle`, `constraint`, `component`, `seq` IDs under correct headings.
- [ ] Define components/sequences/data that will be decomposed later (keep feature-level detail out).
- [ ] Keep alignment with ModKit module structure and secure ORM constraints.

## Validation Checklist

- [ ] `python3 {cypilot_path}/skills/cypilot/scripts/cypilot.py validate --artifact <path>`
- [ ] `make cypilot-validate`
- [ ] Review against `docs/checklists/DESIGN.md`.

## Review Checklist

- [ ] All PRD `fr`/`nfr` referenced under Architecture Drivers
- [ ] All ADR IDs referenced under Architecture Drivers + inline on principles/constraints
- [ ] No spec-level details or decision debates — defer to FEATURE/ADR
- [ ] Architecture alignment, antipatterns, security, API consistency
- [ ] ModKit module structure and secure ORM constraints respected
- [ ] Run `docs/checklists/DESIGN.md` § "PR Review Focus (Design)"
