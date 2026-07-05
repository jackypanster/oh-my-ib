# ADR 0026 — order-preview via non-transmitting whatIf

Status: accepted (2026-07-05, feature order-preview)
Supersedes: none. Relates: ADR 0017 (write-path-safety), ADR 0025 (write-path-semantics-doc),
`docs/write-path-semantics.md` (the `what_if=false` load-bearing row).

## Context

The natural-language → hermes → live-money loop had no preview/confirm step; the operator runs the
Tiger gateway LIVE on `:4001`, so an LLM misparse fires real money blind (PRD §Problem). IB's
`Order.what_if=true` is a non-transmitting margin/commission query — the missing primitive.

## Decision

1. **Preview is a branch at the placement call-site, AFTER `require_live_write_gate`.** The real
   transmit path (`place_with_client`, `what_if=false`) stays **byte-identical** so every existing
   frozen write suite (stk/option/combo/close) remains green; the write gate is **reused unchanged**,
   making preview's gate identical to a real order's *by construction* (zero gate-code change).
2. **Gate identical to a real order (fail-safe), NOT read-shaped.** ✅ human-confirmed. If Tiger
   ignores `what_if`, a "preview" would transmit a real order; gating it exactly like a real order
   guarantees it is never more permissive than the thing it previews. `OMI_ALLOW_LIVE` is session-
   level → near-zero friction. Relaxing to ungated/read-shaped is explicitly DEFERRED and
   evidence-gated on live-acceptance confirming R1 (CONTEXT.md §Reference behavior).
3. **`shape_preview(&Contract, &Order, &OrderState) -> Value` is a pure FROZEN seam** mirroring the
   existing ack seam (trade.rs:51). The OrderState→envelope-key mapping lives inside it and is
   therefore frozen. `OrderState` derives `Default` + has pub fields (ibapi mod.rs:1274), so the
   frozen test constructs a real value literal — a real-type value, not a network mock (honors the
   repo no-mock rule).
4. **`preview_with_client` is a gateway fn, review-by-reading** (= `place_with_client` + `what_if=true`
   + `shape_preview` ack). Not frozen — needs a live gateway, same class as trade.rs:259 fns.
5. **Global `--preview` flag**, `GlobalOpts.preview → Config.preview`, covering all six order verbs.
   ✅ human-confirmed. Reuses all order-arg parsing; a subcommand would duplicate four arg structs.

## Consequences

- One code-change locus: `src/ib/trade.rs` (branch + two seams) plus the flag plumb in
  `cli.rs`/`config.rs`. `main.rs` unchanged.
- The `what_if` premise on Tiger (R1) is NOT frozen-testable → guarded by operator live-acceptance +
  the CONTEXT.md risk register, not a red test. `pipeline-task` MUST NOT freeze gateway behavior.
- Real orders are provably unaffected: the diff adds a branch taken only when `cfg.preview`, and the
  `what_if=false` path is unchanged.

## Rejected alternatives

- **`omi preview <verb>` subcommand** — duplicates four arg structs; a global flag reuses them all.
- **Read-shaped (ungated) preview** — unsafe until R1 is confirmed on Tiger (a refuted R1 would make
  it an ungated real-order path). Deferred, evidence-gated.
- **A `PreviewFields` indirection struct between OrderState and `shape_preview`** — unnecessary:
  `OrderState` is `Default`-constructible with pub fields, so `shape_preview` can take it directly and
  still be frozen. Fewer types, more freeze coverage (the mapping is frozen, not hidden behind
  review-by-reading extraction).
