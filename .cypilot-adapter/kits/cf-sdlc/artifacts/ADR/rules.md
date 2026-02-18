# ADR Rules (CyberFabric)

## Required References (ALWAYS)

ALWAYS open and follow `../../rules.md`

ALWAYS open and use `docs/spec-templates/cf-sdlc/ADR/template.md` as a template
ALWAYS open and use `docs/spec-templates/cf-sdlc/ADR/examples/*.md` as examples

ALWAYS open and follow `docs/checklists/ADR.md` as a quality checklist

## YAML Frontmatter (REQUIRED)

Every ADR file MUST start with YAML frontmatter:

```yaml
---
status: {proposed | accepted | deprecated | superseded}
date: {YYYY-MM-DD}
---
```

## Required Headings (from constraints.json)

| # | Heading | Level | Required | Notes |
|---|---------|-------|----------|-------|
| 1 | H1 title (free-form) | H1 | yes | e.g., `# ADR-0001: {Title}` |
| 2 | `## Context and Problem Statement` | H2 | yes | |
| 3 | `## Decision Drivers` | H2 | yes | |
| 4 | `## Considered Options` | H2 | yes | |
| 5 | `## Decision Outcome` | H2 | yes | |
| 6 | `### Consequences` | H3 | yes | under Decision Outcome |
| 7 | `### Confirmation` | H3 | yes | under Decision Outcome |
| 8 | `## Pros and Cons of the Options` | H2 | yes | |
| 9 | `### {Option Name}` | H3 | no | one per considered option (multiple: allow) |
| 10 | `## More Information` | H2 | no | |
| 11 | `## Traceability` | H2 | no | |

## ID Kinds (from constraints.json)

| Kind | Template | Required | Task | Priority | Heading | to_code |
|------|----------|----------|------|----------|---------|---------|
| `adr` | `cpt-{system}-adr-{slug}` | **yes** | **prohibited** | **prohibited** | H1 title | no |

**Definition form**: plain `**ID**: \`cpt-{system}-adr-{slug}\`` (task/priority prohibited — NO checkbox).

## Cross-Artifact References

| Direction | Target | Coverage | Details |
|-----------|--------|----------|---------|
| ADR → PRD | — | **prohibited** | PRD MUST NOT backtick-reference ADR IDs |
| ADR → DESIGN | `adr` ref | **required** | DESIGN MUST reference every ADR ID under `### Architecture Drivers` |

ADR Traceability section SHOULD include links to related PRD/DESIGN elements as plain references (not backticked IDs in PRD direction).

## Generation Checklist

- [ ] Start with YAML frontmatter (`status`, `date`).
- [ ] Define `adr` ID as plain `**ID**:` form (no checkbox) under H1 title.
- [ ] Include all 8 required headings (Context, Drivers, Options, Outcome with Consequences + Confirmation, Pros/Cons).
- [ ] Capture the problem statement, drivers, ≥2 considered options, and the decision with consequences.
- [ ] Consequences section: list Good/Bad impacts as bullet points.
- [ ] Confirmation section: describe how the decision will be verified.
- [ ] Pros and Cons section: one H3 per considered option with Good/Bad bullet points.
- [ ] Link related DESIGN elements (principles/constraints/components) by ID in Traceability section.
- [ ] Keep ADR decisions immutable once ACCEPTED; allow only structural/syntax/grammar changes.

## Validation Checklist

- [ ] `python3 {cypilot_path}/skills/cypilot/scripts/cypilot.py validate --artifact <path>`
- [ ] `make cypilot-validate`
- [ ] Review against `docs/checklists/ADR.md`

## Review Checklist

- [ ] YAML frontmatter present with valid `status` and `date`
- [ ] `adr` ID in plain form (no checkbox)
- [ ] All 8 required headings present (including Consequences, Confirmation, Pros and Cons)
- [ ] Not already solved by existing ADRs in `docs/adrs/`
- [ ] ≥2 genuinely viable alternatives; rationale traceable to constraints
- [ ] Traceability to DESIGN IDs; PRD must NOT reference ADR IDs
- [ ] Run `docs/checklists/ADR.md` § "PR Review Focus (ADR)"
