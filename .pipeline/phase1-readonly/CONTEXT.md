# CONTEXT — phase1-readonly

Domain glossary + conventions for `oh-my-ib`. Ground all terminology here; do not invent synonyms.

## Interactive Brokers domain

- **TWS API** — Interactive Brokers' socket protocol (binary, over TCP). The `ibapi` crate speaks it.
- **IB Gateway** — IBKR's headless connectivity app (lighter than the full TWS GUI). Must be running
  and logged in for any command to work. Listens on a local TCP port.
- **TWS** — the full desktop trading app; also exposes the API but on different ports. We target the
  Gateway.
- **Ports** — Gateway: **4002 = paper (simulated)**, **4001 = live (real money)**. (TWS uses 7497/7496;
  not our default.) `omi` defaults to 4002; `--live` selects 4001.
- **Paper account** — a simulated-money account mirroring live; the default and the verification target.
- **client_id** — an integer the API uses to distinguish concurrent socket clients of one gateway. Each
  live connection needs a unique id. `omi` default = 100; concurrent invocations must pass `--client-id`.
- **conId (contract id)** — IBKR's unique integer per tradable instrument; the unambiguous handle once
  a symbol is resolved.
- **secType** — security type (STK stock, OPT option, FUT future, CASH forex, …). Phase 1 default STK.
- **SMART** — IBKR's smart-routing pseudo-exchange; the default `exchange` for quotes/contracts.
- **Market-data type** — **live (1)** = real-time (needs a paid subscription), **delayed (3)** =
  ~15-min delayed (free), **frozen (4)** = last close snapshot. `--md-type` default = delayed.
- **NetLiquidation** — total account value if liquidated now. **BuyingPower** — purchasing capacity.
  **AvailableFunds**, **TotalCashValue** — account-summary tags returned by the API.
- **Position** — a held instrument: symbol/conId, quantity, average cost, market value, unrealized PnL.
- **Open order** — a submitted order not yet filled or cancelled (Phase 1 only *reads* these).
- **Pacing / rate limits** — IBKR throttles request frequency; respect it (one-shot per command helps).

## Project conventions

- Binary name **`omi`**; library crate **`oh_my_ib`**; package **`oh-my-ib`**.
- **Read-only Phase 1**: no order-placement code anywhere. Trading is a separate future feature.
- **Public repo**: never commit account ids, tokens, or secrets. Real config lives at
  `~/.config/oh-my-ib/config.toml`.
- **No API keys**: TWS API authenticates via the logged-in Gateway, not tokens.
- **Output**: JSON to stdout by default (agent-parseable); `--format table` for humans; structured
  error envelope to stderr + non-zero exit on failure.
- **Fast-fail**: no swallowed errors; every error carries location + input + concrete cause.
