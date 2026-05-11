# Canonical Error System — Required Fixes

Generated 2026-05-11 from a codebase audit against `docs/arch/errors/{PRD.md,DESIGN.md,ADR/*,categories/*}`.

This document captures every gap between the canonical-error design and the current implementation, with severity, evidence, and proposed actions. Each item is independently decidable — pick, defer, or reject per row.

---

## Landing strategy

Branch `refactor/migrate-to-canonical-errors-pt4` has 8 commits ahead of `main`. **All cleanup lands as new commits on top of the stack.** No history rewriting, no force-push, no re-review of frozen migration commits.

The earlier draft of this document recommended folding the GTS namespace rename into `2674568a` and the oagw TODO reword into `a81e60ab`. That recommendation was withdrawn after pushback: it was framed around a "publish then rename a contract identifier" concern that doesn't actually apply — the branch isn't merged, so internal history doesn't bind the canonical-error contract. The marginal cleanliness gain didn't justify the rebase friction.

**Split the follow-up into independent commits by concern**, in this order:

| Order | Commit subject | Issue(s) | Rough size |
|---|---|---|---|
| 1 | `chore(modkit-errors): standardize GTS namespace to gts.cf.core.<module>.<resource>` | #5 (branch-introduced 3 of 4) | 3-file rename |
| 2 | `docs(oagw): scope tracing-cleanup TODO to Internal/Unknown arms` | part of #2 prep | comment edit |
| 3 | `fix(modkit-errors): map serde_json::Error to InvalidArgument (400) per DESIGN §1.1` | #1 | one impl + tests |
| 4 | `feat(modkit): carry CanonicalError through axum extension so middleware can log diagnostic` | #2 | substantive |
| 5 | `fix(modkit-errors): emit empty object on Problem serialize fallback` | #7 | five lines |
| 6 | `feat(modkit-errors): expose top-level CanonicalError::<category>() constructors; drop ApiGatewayGatewayError` | #4 | medium |
| 7 | `docs(arch/errors): note 413/415 callers will see HTTP 400 under canonical mapping` | #6 | docs |
| 8 | `feat(modkit-errors): impl TryFrom<Problem> for CanonicalError` | #3 | medium, optional |
| 9+ | one commit per SDK migration | #9 | gated on #8 |
| later | `chore(canonical-errors): drop per-arm tracing in non-Internal arms` | #8 | gated on #4 (commit 4 in this list) |

**Summary table**:

| Issue | Disposition | Notes |
|---|---|---|
| #1 serde_json mapping | New commit | Pre-existed in legacy `modkit-canonical-errors` crate |
| #2 Diagnostic unrecoverable | New commit (architecture) + new commit (TODO reword) | TODO reword no longer folded |
| #3 `TryFrom<Problem>` | New commit (or defer) | Design-marked p2 |
| #4 Top-level constructors | New commit | Includes dropping `ApiGatewayGatewayError` |
| #5 GTS namespace | New commit | Branch-introduced renames no longer folded into `2674568a` |
| #6 HTTP status / docs | New commit | Pure docs |
| #7 Problem fallback | New commit | Pre-existing |
| #8 Per-arm tracing | Blocked on #2 | Don't land yet |
| #9 SDK enums | New commit(s), post-#3 | Pre-existing |
| #10 Doc naming | Withdrawn | False positive |

---

## Status legend

| Marker | Meaning |
|---|---|
| **P0 — Drift** | Code disagrees with a documented requirement; wire behavior differs from spec |
| **P1 — Gap** | Documented (often `p2`) capability not yet implemented; blocks downstream work |
| **P2 — Inconsistency** | Internal divergence with no doc story; cleanup/clarification |
| **P3 — Cleanup** | Dead code, latent contract violation, or stale TODO |

---

## 1. `serde_json::Error` maps to `Internal` instead of `InvalidArgument`

- **Severity**: P0 — Drift
- **Evidence**: `libs/modkit-errors/src/error.rs:585-589`
  ```rust
  impl From<serde_json::Error> for CanonicalError {
      fn from(err: serde_json::Error) -> Self {
          Self::__internal(Internal::new(err.to_string()))
              .with_detail("Malformed JSON request body")
      }
  }
  ```
- **Conflict**: `docs/arch/errors/DESIGN.md` §1.1 example (line 28) explicitly states
  `serde_json::Error → InvalidArgument`.
- **Effect**: `?`-propagation on JSON body parsing emits HTTP 500 with `Internal` envelope and detail `"Malformed JSON request body"`, instead of HTTP 400 `InvalidArgument` with `field_violations` context. A pure-client malformed payload is reported as a server fault.
- **Options**:
  1. **(Recommended)** Flip the impl to `Self::__invalid_argument(InvalidArgument::format(err.to_string()))`. Status becomes 400. Audit downstream tests that may assert 500.
  2. Update `DESIGN.md` §1.1 to acknowledge the `Internal` choice (rationale: blanket `From` cannot know whether the JSON is from a request body or an internal pipeline). Less invasive but documents the wire-status degradation.
- **Open question**: Should `From<serde_json::Error>` be removed entirely and require call-site `.map_err(...)`? Blanket impls hide intent; explicit conversion forces handlers to choose 400 vs 500.
- **Landing**: New commit. Verified that `a28c4890^:libs/modkit-canonical-errors/src/error.rs` had the identical impl; the rename did not introduce this drift, so it does not belong in any of the six commits.

---

## 2. `Internal`/`Unknown` diagnostic is unrecoverable in post-response middleware

- **Severity**: P0 — Drift
- **Evidence**:
  - `libs/modkit-errors/src/context.rs:100-103, 320-324` — `Internal.description` and `Unknown.description` are `#[serde(skip)]`.
  - `libs/modkit/src/api/error_layer.rs:38-72, 102-125` — middleware reads response *bytes*, deserializes `Problem`, logs only `status, problem_type, instance, trace_id`.
  - `modules/system/oagw/oagw/src/api/rest/error.rs:112-120` — TODO proposes removing per-arm `tracing::*` calls "now that the middleware logs WARN/ERROR with the trace_id."
- **Conflict**: `DESIGN.md` §3.6 / §3.7 state the middleware logs internal diagnostic information server-side with the `trace_id`. With `description` stripped before serialization, the middleware has nothing to log.
- **Effect**: If the oagw cleanup TODO is executed, the only place where DB errors / panics / unclassified failures are surfaced server-side disappears. Operators lose the ability to correlate `trace_id` → root cause.
- **Options**:
  1. **(Recommended)** Carry `CanonicalError` through an axum `Extension` set inside `IntoResponse for CanonicalError`. The middleware reads the extension and calls `err.diagnostic()` to log the unredacted description. No wire-format change.
  2. Add a private `_description` field to `Problem` and emit it only when a runtime flag like `CF_PROBLEM_DEBUG=1` is set (parallels existing `Problem::from_error_debug`).
  3. Keep per-arm `tracing::error!` in every `From<DomainError> for CanonicalError` site. Reject the oagw TODO and document the convention in `DESIGN.md` §3.6.
- **Cross-reference**: The `oagw` TODO must be reframed/rejected as part of whatever fix is chosen.
- **Landing**:
  - The architectural fix (axum extension carrying `CanonicalError`, middleware reads it, calls `.diagnostic()`) is a **new commit on top of the stack**. Too substantive to fold into `fe7724e8` (which introduced the middleware) without bloating that commit's scope.
  - The misleading TODO at `modules/system/oagw/oagw/src/api/rest/error.rs:114-120` was last edited by `a81e60ab`. Reword it (new commit on top) to scope the cleanup to non-`Internal`/non-`Unknown` arms only. Earlier draft proposed folding via `--fixup=a81e60ab` — withdrawn in favour of an on-top commit per the landing-strategy section.

---

## 3. `TryFrom<Problem> for CanonicalError` is not implemented

- **Severity**: P1 — Gap (design-marked `p2`)
- **Evidence**:
  - `DESIGN.md` §3.3 `cpt-cf-errors-interface-problem-roundtrip` documents the impl signature.
  - No matching `impl TryFrom<Problem>` exists; `rg "TryFrom<Problem>"` returns zero hits.
  - SDKs still hand-roll error enums:
    - `modules/system/authn-resolver/authn-resolver-sdk/src/error.rs` (`AuthNResolverError`)
    - `modules/system/authz-resolver/authz-resolver-sdk/src/{error.rs,pep/enforcer.rs,pep/compiler.rs}`
    - `modules/system/tenant-resolver/tenant-resolver-sdk/src/error.rs`
    - `modules/credstore/credstore-sdk/src/error.rs`
- **Conflict**: `PRD.md` §1.2 cites lossy SDK error reconstruction as a primary motivation; round-tripping is the documented remedy.
- **Effect**: Every SDK consumer (e.g. `api-gateway/src/middleware/auth.rs:252-268`) re-implements `SdkError → CanonicalError` by hand. Adding a new category or SDK error variant requires N call-site updates.
- **Options**:
  1. **(Recommended)** Implement `TryFrom<Problem> for CanonicalError` plus a new `ProblemConversionError` enum. Match on the GTS `type` URI to dispatch into the correct variant. Add a `From<CanonicalError>` for each SDK's legacy enum until the SDK can drop its enum entirely.
  2. Define the dispatch as a derive macro on each context type (avoids a giant match) — heavier infra change.
- **Decision point**: Does this block any current p1 deliverable, or stay at p2?
- **Landing**: New commit (or defer entirely). Touches no file in the current branch's six commits; folding would be artificial.

---

## 4. No top-level `CanonicalError::<category>()` constructors for resource-less use

- **Severity**: P1 — Gap (ergonomics)
- **Evidence**:
  - `libs/modkit-errors/src/builder.rs:584-614` exposes only `internal()`, `service_unavailable()`, `unauthenticated()` at top level.
  - `modules/system/api-gateway/src/middleware/errors.rs:9-16` invents `ApiGatewayGatewayError` purely as an umbrella scope for `invalid_argument` (MIME validation), `resource_exhausted` (rate limit), and `deadline_exceeded` (request timeout).
- **Conflict**: `DESIGN.md` §1.1 ("Non-resource errors (e.g., `service_unavailable`, `unauthenticated`) use `CanonicalError::` constructors directly") does not acknowledge that 13 of 16 categories are unreachable without a resource scope.
- **Effect**: Rate-limit responses are tagged with `resource_type = "gts.cf.core.api_gateway.gateway.v1~"` even though the rejection has nothing to do with a "gateway resource." Consumers parsing `resource_type` get noise. New modules that lack a sensible umbrella scope have to invent one (already happened: `OagwProxyError`, `ApiGatewayGatewayError`).
- **Options**:
  1. **(Recommended)** Add `CanonicalError::invalid_argument()`, `not_found()`, etc. returning builders with `resource_type = None`. Resource-scoped builders remain via `#[resource_error]`. Update DESIGN §3.4 diagram to show two non-resource paths instead of one.
  2. Keep the current shape and add a single first-class `PlatformError` umbrella in `modkit-errors` (`gts.cf.core.platform.v1~`) for these cases. Document that all non-resource non-`internal`/`unauthenticated`/`service_unavailable` errors use it.
  3. Status quo (each module declares its own umbrella). Cheapest, but undocumented and inconsistent.
- **Landing**: New commit. `ApiGatewayGatewayError` was introduced in `fe7724e8`; adding top-level constructors and dropping the umbrella scope would substantially expand that commit's scope. Better as a follow-up that also deletes `ApiGatewayGatewayError`.

---

## 5. GTS namespace inconsistency across modules

- **Severity**: P2 — Inconsistency
- **Evidence** (all `#[resource_error]` declarations):
  - `gts.cf.core.<module>.…` form:
    - `gts.cf.core.am.{tenant,tenant_metadata,conversion_request}.v1~` (account-management)
    - `gts.cf.core.oagw.{proxy,upstream,route,auth_plugin,guard_plugin,transform_plugin}.v1~`
    - `gts.cf.core.resource_group.group.v1~`
    - `gts.cf.core.api_gateway.{route,gateway}.v1~`
    - `gts.cf.core.mini_chat.{chat,message,turn,attachment,model}.v1~`
    - `gts.cf.core.odata.query.v1~`
  - `gts.cf.<module>.<sub>.…` form (missing `core`):
    - `gts.cf.simple_user_settings.settings.user.v1~`
    - `gts.cf.file_parser.parser.file.v1~`
    - `gts.cf.nodes_registry.registry.node.v1~`
    - `gts.cf.types_registry.registry.type.v1~`
- **Conflict**: `modkit-errors-macro/src/lib.rs:252-327` validates structure only (≥ 5 segments, lowercase, `v<digits>` version); no convention enforcement. No catalog rule in `DESIGN.md` mandates either shape.
- **Effect**: Consumers cannot do prefix-based filtering reliably (e.g. "all CF-core errors"). GTS Type Registry registration may diverge.
- **Options**:
  1. **(Recommended)** Standardize on `gts.cf.core.<module>.<resource>.v1~` for first-party modules. Migrate the four divergent ones. Document the convention in `DESIGN.md` §3.5.
  2. Keep diverse forms but document an explicit prefix taxonomy (`gts.cf.core.*` = first-party, `gts.cf.<x>.*` = experimental/external).
  3. Add a lint to `modkit-errors-macro` that warns on non-`gts.cf.core.*` prefixes outside an opt-in attribute.
- **Migration risk**: Changing a GTS identifier is a breaking change per `DESIGN.md` §2.2 (`cpt-cf-errors-constraint-error-contract-stability`). Coordinate with consumers.
- **Landing**:
  - `gts.cf.file_parser.parser.file.v1~` was introduced by `b9738f36` (already in `main`). Out of branch scope; rename via new commit if/when consumers can absorb the break.
  - `gts.cf.simple_user_settings.…`, `gts.cf.nodes_registry.…`, `gts.cf.types_registry.…` were introduced by `2674568a`. New commit on top renames all three to the `gts.cf.core.<module>.<resource>.v1~` form. Earlier draft proposed folding via `--fixup=2674568a` — withdrawn: the branch is unmerged so internal commit ordering doesn't constrain the public contract, and the rebase friction outweighs the marginal cleanliness gain.

---

## 6. HTTP-status / canonical-category mismatches surprise HTTP-aware clients

- **Severity**: P2 — Inconsistency (vs HTTP convention; matches DESIGN table)
- **Evidence**:
  - `modules/system/oagw/oagw/src/api/rest/error.rs:206-210`: `PayloadTooLarge` → `out_of_range` ⇒ HTTP 400 (HTTP convention: 413).
  - `modules/mini-chat/mini-chat/src/api/rest/error.rs:149-153, 300-304, 320-344`: `FileTooLarge`, `TooManyImages`, `ContextBudgetExceeded`, `InputTooLong` → `out_of_range` ⇒ 400.
  - `modules/system/api-gateway/src/middleware/mime_validation.rs:51-56` returns `invalid_argument` (400), while `libs/modkit/src/api/operation_builder.rs:1538-1547` exposes `error_415(...)` declaring 415 in OpenAPI. Doc ≠ wire.
- **Conflict**: `DESIGN.md` §1.2 fixes `invalid_argument`/`out_of_range`/`failed_precondition` all at HTTP 400. The mappings are spec-compliant; the surprise is for clients that branch on HTTP status alone.
- **Options**:
  1. **(Recommended, cheap)** Document the deviations explicitly in `docs/arch/errors/categories/{03-invalid-argument,11-out-of-range}.md` ("HTTP 413/415 callers will see 400; branch on the GTS `type` field, not status.").
  2. Split `out_of_range` into `out_of_range_value` (400) and `payload_too_large` (413). Adds a 17th category — breaks "finite vocabulary" assumption unless added carefully.
  3. Change MIME validation to use HTTP 415. Requires a new canonical category (`unsupported_media_type`) or a special-case status override — both invasive.
- **Recommendation**: Option 1. The wire contract is the GTS URI, not the HTTP status.
- **Landing**: New commit. Pure docs (`docs/arch/errors/categories/{03,11}.md`) spanning oagw, mini-chat, and api-gateway behaviors; one doc commit is cleaner than three fixups.

---

## 7. `Problem::from_error` fallback writes a non-object `context`

- **Severity**: P3 — Cleanup (latent contract violation)
- **Evidence**: `libs/modkit-errors/src/problem.rs:122-137`
  ```rust
  impl From<CanonicalError> for Problem {
      fn from(err: CanonicalError) -> Self {
          match Problem::from_error(&err) {
              Ok(p) => p,
              Err(ser_err) => Problem {
                  // …
                  context: serde_json::Value::String(ser_err.to_string()),
              },
          }
      }
  }
  ```
- **Conflict**: `DESIGN.md` §3.3 base schema declares `context: object`. Built-in context types are plain structs that should never fail to serialize, so this fallback is dead — but if it ever fires, the wire body violates the schema.
- **Options**:
  1. **(Recommended)** Log the serde error at `error!` and emit `context: serde_json::Value::Object(Default::default())` (empty object). Matches schema; behaviour stays defensive.
  2. Drop the fallback; let the `IntoResponse` impl panic (the existing fallback body in `IntoResponse` already handles serialization failure).
  3. Keep as-is and update `DESIGN.md` to acknowledge the union type for `context` in catastrophic-failure mode.
- **Landing**: New commit. Pre-existed in the legacy crate; rename preserved it.

---

## 8. Per-arm `tracing::*` calls are inconsistently kept across modules

- **Severity**: P3 — Cleanup
- **Evidence**: Every migrated module's `From<DomainError> for CanonicalError` has `#[allow(clippy::cognitive_complexity)]` because of `tracing::warn!`/`error!` calls inside each arm. The oagw module has a documented TODO to remove them (`modules/system/oagw/oagw/src/api/rest/error.rs:112-120`).
- **Status**: This cleanup is **blocked by issue #2**. Per-arm logging is currently the only mechanism that surfaces `Internal::description` server-side.
- **Decision rule**: Once #2 is resolved:
  - Drop `tracing::*` from arms that produce `not_found`, `invalid_argument`, `permission_denied`, etc. — the wire envelope already conveys everything needed and the middleware log already fires.
  - **Keep** `tracing::error!`/`warn!` in arms that produce `internal`, `unknown`, `service_unavailable` (since description / cause is dropped on the wire).
- **Reject** any cleanup PR that lands before #2 is fixed.
- **Landing**: Do not land in this branch. Gated on #2.

---

## 9. SDK error enums still exist alongside canonical errors

- **Severity**: P1 — Gap (downstream of #3)
- **Evidence**: See files listed in #3.
- **Conflict**: `PRD.md` §1.2 ("SDK clients ... maintain their own ad-hoc error enums that are lossy, manual reconstructions of server responses") is unresolved.
- **Effect**: Cross-module callers (e.g. account-management's `From<authz_resolver_sdk::EnforcerError> for DomainError` at `modules/system/account-management/account-management/src/domain/error.rs:217-263`) re-translate SDK errors into domain types that re-translate again into canonical. Two boundary mappings instead of one.
- **Options** (after #3 lands):
  1. Drop the SDK-specific error enums; export `CanonicalError` as the SDK error type (account-management has already done this: `account-management-sdk/src/lib.rs:62` re-exports `CanonicalError as AccountManagementError`). Apply to authn-resolver-sdk, authz-resolver-sdk, tenant-resolver-sdk, credstore-sdk.
  2. Keep SDK enums but auto-derive `From<SdkError> for CanonicalError` via a derive macro in `modkit-errors-macro`. Less disruption to existing consumers.
- **Sequencing**: Cannot start before #3.
- **Landing**: New commit (or commits, one per SDK). Pre-existed in `main`; not in branch scope.

---

## 10. ~~`DESIGN.md` references `cf-modkit-errors` and `modkit-errors` inconsistently~~ — WITHDRAWN

- **Status**: False positive identified on second pass.
- **Re-evaluation**: `DESIGN.md` (as of `a28c4890`) consistently uses `cf-modkit-errors` for the Cargo package name (e.g. §2.2 breaking-change policy, §3.10 CI rules), `modkit_errors` for Rust `use` paths (e.g. §1.1, §2.2 example code), and `libs/modkit-errors` for directory references (e.g. §3.1 Location field). These are three distinct correct references to the same artifact, not three competing names. Rename commit handled this correctly.
- **Action**: None.

---

## Suggested ordering

See the **Landing strategy** table at the top of this document. All fixes land as new commits on top of the existing 8-commit stack — no rebase, no fixups. Order is reproduced here for convenience:

1. GTS namespace standardization (#5, branch-introduced 3 of 4).
2. oagw TODO reword (prep for #2).
3. `serde_json::Error → InvalidArgument` (#1).
4. Middleware diagnostic recovery via axum extension (#2).
5. `Problem` fallback emits empty object (#7).
6. Top-level `CanonicalError::<category>()` constructors + drop `ApiGatewayGatewayError` (#4).
7. Categories docs clarification for 413/415 (#6).
8. `TryFrom<Problem> for CanonicalError` (#3, optional / p2).
9. SDK enum migration, one commit per SDK (#9, gated on #3).
10. Drop per-arm tracing in non-`Internal`/non-`Unknown` arms (#8, gated on #2).
11. `gts.cf.file_parser.*` rename (#5, remainder) — only if/when consumers can absorb the contract break; this one pre-existed in `main` so it's a separate decision from the rest of #5.

## Open questions

- Is HTTP-status fidelity (413, 415, …) a requirement, or is the GTS `type` URI authoritative? Affects #6.
- Should blanket `From` impls (`io::Error`, `serde_json::Error`, `DbErr`) exist at all, given they encode policy decisions (which category)? Affects #1 and any future `From<sqlx::Error>` / `From<rusqlite::Error>`.
- Are the legacy modules (credstore, authn-resolver, authz-resolver, tenant-resolver) on the migration plan, or do they stay pre-canonical until they grow a public HTTP surface? Affects #3 / #9 priority.
