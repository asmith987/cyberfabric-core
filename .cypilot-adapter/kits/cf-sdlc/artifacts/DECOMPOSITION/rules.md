# DECOMPOSITION Rules (CyberFabric)

## Required References (ALWAYS)

ALWAYS open and follow `../../rules.md`

ALWAYS open and use `docs/spec-templates/cf-sdlc/DECOMPOSITION/template.md` as a template
ALWAYS open and use `docs/spec-templates/cf-sdlc/DECOMPOSITION/examples/example.md` as an example
ALWAYS open and follow `docs/checklists/DECOMPOSITION.md` as a quality checklist

## Required Headings (from constraints.json)

| # | Heading | Level | Required | Notes |
|---|---------|-------|----------|-------|
| 1 | H1 title (free-form) | H1 | yes | e.g., `# Decomposition: {System Name}` |
| 2 | `## Overview` | H2 | yes | |
| 3 | `## Entries` | H2 | yes | |
| 4 | `### {Feature Title}` | H3 | yes | one per feature (multiple: allow) |
| 5 | `## Feature Dependencies` | H2 | no | |

## Document Structure

```markdown
# Decomposition: {System Name}

- [ ] `p1` - **ID**: `cpt-{system}-status-{slug}`      ← status ID under H1

## 1. Overview
{decomposition strategy}

## 2. Entries

### 1. {Feature Title} - {PRIORITY}
- [ ] `p1` - **ID**: `cpt-{system}-feature-{slug}`      ← feature ID def
- **Purpose**: ...
- **Depends On**: ...
- **Scope**: ...
- **Requirements Covered**: `cpt-{system}-fr-*`, `cpt-{system}-nfr-*`
- **Design Principles Covered**: `cpt-{system}-principle-*`
- **Design Constraints Covered**: `cpt-{system}-constraint-*`
- **Design Components**: `cpt-{system}-component-*`
- **Sequences**: `cpt-{system}-seq-*`
- **Data**: `cpt-{system}-dbtable-*`

## 3. Feature Dependencies
{dependency graph}
```

## ID Kinds (from constraints.json)

| Kind | Template | Required | Task | Priority | Heading | to_code |
|------|----------|----------|------|----------|---------|---------|
| `status` | `cpt-{system}-status-{slug}` | **yes** | **required** | **required** | **H1 title** | no |
| `feature` | `cpt-{system}-feature-{slug}` | **yes** | **required** | **required** | feature entry (H3) | no |

Both kinds use checkbox definition form: `- [ ] \`p1\` - **ID**: \`cpt-...\``

**`status`** is placed directly under the H1 title (before `## Overview`).

## Cross-Artifact References

### Inbound (IDs defined elsewhere, referenced in DECOMPOSITION feature entries)

| Source | Kind | Coverage | Where |
|--------|------|----------|-------|
| PRD | `fr` | optional | feature entry "Requirements Covered" |
| PRD | `nfr` | optional | feature entry "Requirements Covered" |
| DESIGN | `component` | **required** | feature entry "Design Components" |
| DESIGN | `principle` | optional | feature entry "Design Principles Covered" |
| DESIGN | `constraint` | optional | feature entry "Design Constraints Covered" |
| DESIGN | `seq` | optional | feature entry "Sequences" |
| DESIGN | `db`/`dbtable` | optional | feature entry "Data" |

**Critical**: every DESIGN `component` MUST be referenced in at least one feature entry.

### Outbound (DECOMPOSITION IDs referenced downstream)

| Kind | Target | Coverage | Target Heading |
|------|--------|----------|----------------|
| `feature` | FEATURE | **required** | FEATURE H1 title (checkbox reference) |

**Critical**: every `feature` ID MUST be covered by a FEATURE artifact. The FEATURE file places a tracked reference `- [x] \`pN\` - \`cpt-{system}-feature-{slug}\`` directly under its H1.

## FEATURE File Creation

When generating DECOMPOSITION, create stub FEATURE files:

- Path: `features/NNNN-cpt-{system}-feature-{slug}.md`
- Minimal content: H1 title + `featstatus` ID + `feature` reference + `## 1. Feature Context` with subsections

## Generation Checklist

- [ ] Define `status` ID under H1 title (checkbox form).
- [ ] Decompose DESIGN components/sequences/data into features with high cohesion and clear boundaries.
- [ ] Ensure 100% coverage: every DESIGN `component` appears in at least one feature entry.
- [ ] Avoid overlap: design elements should not be duplicated across features without explicit reason.
- [ ] Assign priorities (`p1`-`p9`) and keep dependencies explicit and acyclic.
- [ ] Each feature entry: ID def (checkbox) + Purpose + Depends On + Scope + covered IDs.
- [ ] Create stub FEATURE files in `features/` for each entry.

## Validation Checklist

- [ ] `python3 {cypilot_path}/skills/cypilot/scripts/cypilot.py validate --artifact <path>`
- [ ] `make cypilot-validate`
- [ ] Review against `docs/checklists/DECOMPOSITION.md`.

## Review Checklist

- [ ] `status` ID present under H1 with checkbox form
- [ ] At least one `feature` ID; each with Purpose, Scope, Dependencies, covered IDs
- [ ] 100% DESIGN `component` coverage; no overlap without explicit reason
- [ ] No circular dependencies — valid DAG; consistent granularity
- [ ] No implementation details, requirements defs, or decision debates
- [ ] Run `docs/checklists/DECOMPOSITION.md` for full validation
