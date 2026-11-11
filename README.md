# percolator-agent-registry

Open agent ⇄ token/market pairing registry for Metaplex Core agent assets. Part of the $CLAWD ecosystem (no new tickers).
**Unaudited. Deployed on test chains only (clawd chain; Solana devnet pending faucet funds). No mainnet deploy.**

Any agent (a Core asset) can be paired with any SPL / Token-2022 mint, pump.fun bonding curve, PumpSwap pool, Raydium
(AMM v4 / CPMM / CLMM / LaunchLab) or Meteora (DLMM / DAMM v1 / v2 / DBC) pool, Genesis launch, Percolator market, or
Phoenix / other perps market. dev's own agents pair only with $CLAWD `8cHzQHUS2s2h8TzCmfqPKYiM4dSt4roa3n7MyRLApump`.

## Clean-room statement
Written from scratch in Pinocchio from public account layouts and observed behaviour. No code from `mpl-agent`,
`mpl-core` or any other Metaplex repository is copied or linked: the Core asset is parsed from its raw bytes
(key `1` at byte 0, owner at bytes 1..33) and the asset-signer PDA is derived from its public seeds
(`["mpl-core-execute", asset]` under `CoREEN…`). The Core program binary used by one test is dumped from devnet at
test time (`scripts/fetch-fixtures.sh`) and is never committed. Licensed MIT.

## Accounts
| Account | Seeds | Size | Notes |
|---|---|---|---|
| RegistryConfig | `["config"]` | 808 B | admin, paused flag, 16 allowlist slots (program, kind, optional ≤8-byte magic at an offset) |
| PairingV1 | `["pairing", asset, target]` | **208 B** | one per (agent, target); many targets per agent |
| PrimaryPointer | `["primary", asset]` | 48 B | the agent's canonical pairing |

PairingV1: `key u8 | bump u8 | kind u8 | flags u8 | pad[4] | asset | target | target_program | resolved_mint | set_by | slot u64 | reserved[32]`.
Flags: `LOCKED 1, PRIMARY 2, VERIFIED_MINT 4, UNVERIFIED_TARGET 8, VIA_ASSET_SIGNER 16`.

Kinds and on-chain checks:
| kind | value | check |
|---|---|---|
| Spl | 0 | owner Tokenkeg, len 82, initialized; resolved = target |
| Token2022 | 1 | owner Tokenz, mint layout (len 82 or account-type byte 165 = 1), initialized |
| PumpCurve | 2 | owner `6EF8…`; resolved mint required, PDA `["bonding-curve", mint]` re-derived |
| PumpSwap | 3 | owner `pAMM…`; resolved = base mint @43 (or must equal base/quote) |
| Raydium | 4 | owner ∈ AMM v4 / CPMM / CLMM / LaunchLab (+ allowlist) |
| Meteora | 5 | owner ∈ DLMM / DAMM v1 / DAMM v2 / DBC (+ allowlist) |
| Genesis | 6 | owner `GNS1…`, len ≥ 136, disc 18, funding mode 0; resolved = base mint @40 |
| Percolator | 7 | owner ∈ allowlist, optional magic (devnet `Anw121…` "PERCOLAT" @0; clawd `6Grr…` "61VCREP" @1) |
| Perp | 8 | owner Phoenix `PhoeNiX…` (+ allowlist) |
| Other | 9 | account must exist; flagged UNVERIFIED |

## Instructions (1-byte tag)
| tag | name | accounts | data |
|---|---|---|---|
| 0 | InitConfig | config w, payer s w, upgrade_authority s, program_data, system | admin[32] — only the program's upgrade authority can init |
| 1 | Pair | config, asset, target, pairing w, payer s w, authority s, system, [primary w], [current primary pairing w] | kind u8, opts u8 (1 lock, 2 primary), resolved_mint[32] |
| 2 | Unpair | pairing w, asset, authority s, recipient w, [primary w] | — closes and refunds rent; rejected if LOCKED |
| 3 | Lock | pairing w, asset, authority s | — one-way |
| 4 | SetPrimary | pairing w, asset, authority s, primary w, payer s w, system, [current w] | — rejected if current primary is LOCKED |
| 5 | AdminSetAllowlist | config w, admin s | index, kind, magic_off, magic_len, magic[8], program[32] |
| 6 | AdminUpdate | config w, admin s | new_admin[32], paused u8 |

**Authority** for 1–4 is either the asset owner signing directly, or the Core asset-signer PDA (wrap the instruction in
Core `ExecuteV1`; `coreExecuteIx` in the client). Ownership is re-read on every call, so pairings follow asset transfers.
Pre-funded PDAs cannot block creation (top-up + allocate + assign path).

## Build / test
```bash
cd program && cargo build-sbf                # → target/deploy/percolator_agent_registry.so (30,832 B)
./scripts/fetch-fixtures.sh                  # optional: Core .so for the Execute test
cd program-tests && cargo test               # 14 LiteSVM tests
cd client && npm i && npm test && npx tsc -p .
REGISTRY_KEYPAIR=/tmp/<disposable>.json CLUSTER=clawd|devnet|devnet-fork npx tsx scripts/e2e.ts
```
The e2e script refuses mainnet (genesis check) and refuses keypairs outside `/tmp`.

## Deployments (test chains only)
See `DEPLOYMENTS.md`.
