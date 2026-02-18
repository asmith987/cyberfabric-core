# FEATURE Rules (CyberFabric)

## Required References (ALWAYS)

ALWAYS open and follow `../../rules.md`

ALWAYS open and use `docs/spec-templates/cf-sdlc/FEATURE/template.md` as a template
ALWAYS open and use `docs/spec-templates/cf-sdlc/FEATURE/examples/*.md` as examples
ALWAYS open and follow `docs/checklists/FEATURE.md` as a quality checklist

## Required Headings (from constraints.json)

| # | Heading | Level | Required | Notes |
|---|---------|-------|----------|-------|
| 1 | H1 title (free-form) | H1 | yes | e.g., `# Feature: {Name}` |
| 2 | `## Feature Context` | H2 | yes | |
| 3 | `### Overview` | H3 | yes | |
| 4 | `### Purpose` | H3 | yes | |
| 5 | `### Actors` | H3 | yes | |
| 6 | `### References` | H3 | yes | |
| 7 | `## Actor Flows (CDSL)` | H2 | yes | |
| 8 | `### {Flow Name}` | H3 | no | one per flow (multiple: allow) |
| 9 | `## Processes / Business Logic (CDSL)` | H2 | yes | |
| 10 | `### {Process Name}` | H3 | no | one per algo (multiple: allow) |
| 11 | `## States (CDSL)` | H2 | no | |
| 12 | `### {State Machine Name}` | H3 | no | one per state machine (multiple: allow) |
| 13 | `## Definitions of Done` | H2 | yes | |
| 14 | `### {DoD Entry}` | H3 | yes | one per dod (multiple: allow) |
| 15 | `## Acceptance Criteria` | H2 | yes | |

## Document Structure (H1 area)

```markdown
# Feature: {Name}

- [ ] `p2` - **ID**: `cpt-{system}-featstatus-{slug}`    ← featstatus definition (checkbox)

- [x] `p1` - `cpt-{system}-feature-{slug}`               ← feature reference back to DECOMPOSITION (checkbox)

## 1. Feature Context
...
```

**Both IDs are placed directly under the H1 title** (before `## Feature Context`):
1. `featstatus` — own feature status (definition, checkbox form)
2. `feature` — tracked reference back to the DECOMPOSITION entry (reference, checkbox form)

## ID Kinds (from constraints.json)

| Kind | Template | Required | Task | Priority | Heading | to_code |
|------|----------|----------|------|----------|---------|---------|
| `featstatus` | `cpt-{system}-featstatus-{slug}` | **yes** | **required** | **required** | **H1 title** | no |
| `flow` | `cpt-{system}-flow-{slug}` | no | **required** | **required** | `### {Flow Name}` | **yes** |
| `algo` | `cpt-{system}-algo-{slug}` | no | **required** | **required** | `### {Process Name}` | **yes** |
| `state` | `cpt-{system}-state-{slug}` | no | **required** | **required** | `### {State Machine}` | **yes** |
| `dod` | `cpt-{system}-dod-{slug}` | **yes** | **required** | **required** | `### {DoD Entry}` | **yes** |

All FEATURE ID kinds require task+priority → use checkbox definition form: `- [ ] \`p1\` - **ID**: \`cpt-...\``

**`to_code: true`** means these IDs drive code traceability via `@cpt-*` markers.

## Cross-Artifact References

### Inbound (IDs from DECOMPOSITION, referenced in FEATURE)

| Source | Kind | Coverage | Where |
|--------|------|----------|-------|
| DECOMPOSITION | `feature` | **required** | H1 title area (tracked checkbox reference) |

**Critical**: FEATURE MUST include a tracked checkbox reference to its DECOMPOSITION `feature` ID under H1. This satisfies the DECOMPOSITION→FEATURE coverage requirement.

### Inbound (IDs from PRD/DESIGN, referenced in FEATURE for context)

| Source | Kind | Where in FEATURE |
|--------|------|------------------|
| PRD | `fr`/`nfr` | `### Purpose` (optional) |
| DESIGN | `principle` | `### Purpose` (optional) |
| DESIGN | `constraint` | `## Definitions of Done` (optional) |
| DESIGN | `db`/`dbtable` | `## Definitions of Done` (optional) |

### Outbound (FEATURE IDs traced to CODE)

| Kind | Mechanism |
|------|-----------|
| `flow` | `@cpt-flow:{id}:p{N}` / `@cpt-begin`/`@cpt-end` markers in code |
| `algo` | `@cpt-algo:{id}:p{N}` / `@cpt-begin`/`@cpt-end` markers in code |
| `state` | `@cpt-state:{id}:p{N}` / `@cpt-begin`/`@cpt-end` markers in code |
| `dod` | `@cpt-req:{id}:p{N}` / `@cpt-begin`/`@cpt-end` markers in code |

## Checkbox Cascade

```
CODE markers exist for flow/algo/state/dod
    ↓
FEATURE: flow/algo/state/dod → [x]
    ↓
FEATURE: ALL IDs [x] → featstatus → [x]
    ↓
DECOMPOSITION: feature → [x]
```

## Generation Checklist

- [ ] Define `featstatus` ID under H1 (checkbox form).
- [ ] Add tracked `feature` reference under H1 (checkbox reference back to DECOMPOSITION).
- [ ] Include all required headings (see table above).
- [ ] Define at least one `dod` ID (checkbox form, `to_code: true`).
- [ ] Define flows/algorithms/states at implementable detail level (inputs/outputs, errors, edge cases).
- [ ] Write DoD criteria that are testable and map to `make test`/integration/E2E expectations.
- [ ] Reference relevant PRD/DESIGN IDs in Purpose and DoD sections for context.
- [ ] Keep details consistent with ModKit patterns, secure ORM, and OData/OpenAPI conventions.

## Validation Checklist

- [ ] `python3 {cypilot_path}/skills/cypilot/scripts/cypilot.py validate --artifact <path>`
- [ ] `make cypilot-validate`
- [ ] Review against `docs/checklists/FEATURE.md`.

## Review Checklist

- [ ] `featstatus` ID present under H1 with checkbox form
- [ ] `feature` reference present under H1 with checkbox form
- [ ] At least one `dod` ID with checkbox form
- [ ] Flows cover happy path, errors, edge cases; algorithms testable
- [ ] No type redefinitions, new API endpoints, code snippets, or decision debates
- [ ] DoD criteria testable and map to `make test`/integration/E2E
- [ ] ModKit patterns, secure ORM, security checks in flows
- [ ] Run `docs/checklists/FEATURE.md` for full validation
