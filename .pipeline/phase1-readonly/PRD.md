# PRD — phase1-readonly

Feature: a read-only IBKR command-line tool, `omi`, driven by an LLM agent over chat.
Status: decision-complete (grilling done in the originating `/think` session; decisions below are locked).

## Problem

The operator wants to monitor an Interactive Brokers account from the terminal by chatting
with an agent (Claude Code), instead of clicking through TWS. The agent needs a deterministic,
scriptable tool it can invoke and whose output it can parse reliably. No such tool exists in Rust
for this operator's workflow.

## Goal

Ship `omi`, a non-interactive Rust CLI that connects to a local IB Gateway over the TWS API
(`ibapi` crate) and **reads** account state on demand. Each invocation: connect → request → emit
JSON → disconnect. The agent runs it via shell, parses JSON, reports to the operator in chat.

## Success criteria (acceptance, against a PAPER account)

1. `omi health` connects to IB Gateway and prints connection status, server version, and the
   accessible account id(s); exits 0. With the gateway down it exits non-zero and prints a
   structured `{"error":{...}}` to stderr.
2. `omi account` returns net liquidation, total cash, buying power, available funds (JSON).
3. `omi positions` returns held positions (symbol, qty, avg cost, market value, unrealized PnL).
4. `omi orders` returns open (not-yet-filled) orders.
5. `omi quote AAPL --md-type delayed` returns a delayed quote with a field marking it delayed.
6. `omi contract AAPL` resolves the contract (conId, exchange, currency, secType).
7. `omi history AAPL --bar 1d --duration 1M` returns historical daily bars.
8. `--format table` renders the same data human-readably for any of the above.
9. `cargo build`, `cargo clippy -- -D warnings`, `cargo test` all pass.

## Scope (Phase 1)

- Rust binary crate `oh-my-ib`, binary name `omi`.
- `ibapi` crate, **sync (blocking) client**: `ibapi = { version = "3.1", default-features = false, features = ["sync"] }`.
- Subcommands: `health`, `account`, `positions`, `orders`, `quote`, `contract`, `history`.
- Stateless per invocation: connect, request, print, disconnect. No daemon, no long-lived connection.
- Output contract: `--format json` (default) emits one JSON object/array to stdout; `--format table`
  for humans. Errors → non-zero exit + `{"error":{code,message,context}}` to stderr.
- Config file `~/.config/oh-my-ib/config.toml` (host, port, client_id, default_account, md_type,
  optional risk caps for later phases); every field overridable by a CLI flag.
- Global flags: `--host` (default 127.0.0.1), `--port` (default 4002 = paper), `--client-id`
  (default 100), `--account`, `--format json|table`, `--md-type live|delayed|frozen` (default delayed),
  `--live` (selects port 4001; read-only is permitted on live), `--timeout`.

## Non-scope (explicitly NOT in Phase 1)

- No order placement / modify / cancel — no such code path exists in the binary (structurally read-only).
- No daemon / streaming / real-time subscriptions / market scanner / Flex queries.
- No IB Gateway auto-login (IBC).
- No Web API / OAuth / Client Portal — TWS API only.
- No GUI. No self-built socket protocol (the `ibapi` crate owns the wire protocol).
- No mocks/stubs — verification runs against a real paper account.

## Resolved decisions (locked)

- D1 **Interface = TWS API via `ibapi` + IB Gateway.** Rationale: native Rust, MIT, actively
  maintained (v3.1, updated 2026-06), most complete feature set; the crate already encapsulates the
  socket protocol. Web API rejected (heavier session model, fewer features, same gateway requirement).
- D2 **Sync (blocking) client, not async.** Short-lived process, sequential requests, no concurrency
  → blocking is the simplest correct shape; avoids pulling in a Tokio runtime.
- D3 **Stateless connect-per-command.** Robust, no residual connections; fast enough for `ibapi`.
  Fixed default `client_id=100`; concurrent invocations must pass distinct `--client-id`.
- D4 **JSON-first output.** The agent parses JSON; humans get `--format table`. Fast-fail with
  structured errors + non-zero exit codes (no swallowed exceptions; context = location/input/cause).
- D5 **Paper default, live opt-in.** Default port 4002; `--live` → 4001 (read-only on live is allowed).
  Write operations (future phases) additionally gated by `OMI_ALLOW_LIVE=1`.
- D6 **Delayed market data default.** Real-time quotes need a paid subscription; default
  `--md-type delayed` avoids errors for unsubscribed users; output marks data as delayed vs live.
- D7 **Minimal dependencies:** `ibapi`, `clap` (derive), `serde`, `serde_json`, `anyhow`, `toml`.

## Dependencies / prerequisites (operator-owned)

- IBKR **Pro** account (Lite has API/market-data limits) + a **paper** account — VERIFY before first run.
- **IB Gateway** installed and logged in during use (manual daily login/2FA; IBC automation deferred).
- IB Gateway → Configure → Settings → API: enable "ActiveX and Socket Clients", set port
  (4002 paper / 4001 live), add `127.0.0.1` to Trusted IPs, optionally enable "Read-Only API" as a
  second defense layer during Phase 1.
- No API keys/tokens/third-party accounts (TWS API auth = the logged-in gateway).

## Verification

- Build/lint/test: `cargo build`, `cargo clippy -- -D warnings`, `cargo test`.
- Manual acceptance (paper gateway on :4002): run criteria 1–8 above.
- Failure path: stop the gateway → `omi health` exits non-zero with a structured error.

## Risks / fragile assumptions

- **Load-bearing assumption:** IB Gateway stays up and logged in. Mitigated by connect-per-command +
  `omi health` explicit liveness probe, so disconnects surface fast instead of hanging silently.
- Market-data entitlements vary by account; delayed default avoids hard failures.
