# Access and Compliance Checklist

Date: 2026-05-17
Status: Pre-live

## Credential Handling

Live secrets were provided in chat on 2026-05-17. Treat all provided values as compromised.

Required before any live API call or live order placement:

- Rotate the Polymarket wallet private key or move funds to a new wallet.
- Revoke and recreate Polymarket CLOB API credentials.
- Revoke and recreate Kalshi API key and RSA private key.
- Rotate Chainlink, Candlestick, and Dome credentials.
- Store new values only in a local `.env` file or a secret manager.
- Do not commit `.env` files.
- Do not paste live private keys or API secrets into chat again.

## Current Provider Inventory

| Provider | Purpose | Config variables | Live use allowed now? |
|---|---|---|---|
| Polymarket | CLOB trading and wallet signing | `POLY_PRIVATE_KEY`, `POLY_API_KEY`, `POLY_API_SECRET`, `POLY_API_PASSPHRASE` | No, rotate first |
| Chainlink | Data streams / candlestick data | `CHAINLINK_USERNAME`, `CHAINLINK_PASSWORD`, `CANDLESTICK_API_KEY` | No, rotate first |
| Dome | Real-time tape WebSocket | `DOME_API_KEY` | No, rotate first |
| Kalshi | REST/WebSocket trading API | `KAL_API_KEY`, `KAL_API_SECRET`, `KAL_API_BASE`, `KAL_API_WS` | No, rotate first |

## MVP Access Decisions Needed

- Polymarket master wallet addresses to monitor.
- Polymarket follower wallet address after key rotation.
- Kalshi master account access mode:
  - authorized read-only API key
  - authorized subaccount
  - unavailable for MVP
- Kalshi follower account mode:
  - demo first
  - production dry-run only
  - production small-notional after approval
- Initial MVP provider priority:
  - Polymarket first
  - Kalshi first
  - both in dry-run

## Live Trading Gate

Live trading remains disabled unless all are true:

- `LIVE_TRADING_ENABLED=true`.
- Rotated credentials are installed outside source control.
- Global stop-copy has been tested.
- Daily max loss is configured.
- Per-trade max notional is configured.
- Dry-run has run without duplicate copy intents.
- Reconciliation is working for the target platform.

## Initial Safe Defaults

```text
LIVE_TRADING_ENABLED=false
EDGECOPY_EXECUTION_PROVIDER=dry_run
KAL_API_ENVIRONMENT=development
EDGECOPY_KALSHI_ENVIRONMENT=development
```
