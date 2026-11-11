# Deployments (test chains only)

Program ID (same keypair on every test chain): `A5tHHrrXcYaswtqHzh7V711CVNKPt2mLRou5yGy3ADJm`  
Binary: `percolator_agent_registry.so` **30,832 bytes**, sha256 `05b72ec19fbe6d5df6ed0a7947e476585f93251039203fec7fb3665f441aa203`  
Upgrade authority (test chains only): disposable faucet-funded key `6ThLf4zU3HNxrPjNgr9NzSmgxXNEJaagzVUm8gQwBc4K` (lives only in /tmp on the build box; never a house/dev key). Registry admin = same key.

## clawd chain (deployed 2026-11-11 ~3:49 PM ET)

| What | Value |
|---|---|
| RPC | https://rpc.clawdchain.trade |
| Deploy tx | [5HU9FgJm…](https://explorer.clawdchain.trade/tx/5HU9FgJmiN9m3tk86sMbf3HN4jWesK3S2bwfNoXAZc6t9Seo3GJpR1E71894Jo4BCvzj3qB7S4P2WzLmgt8ABjiQ) |
| ProgramData | `Cw8QDm7T8C8tRxSsuqzs9Qj7tfB7PoUUwtP5Mr3AECq7`, max_len 61,664 (2× headroom), 0.43038552 test CLAWD rent |
| Deploy total | ≈0.4317 test CLAWD (faucet; no value) |
| Test agent (Core asset) | `zDJgSKAYiJcJD7nKyHdUmKUJ1W37ABcRyz3MN41qgAR` (asset signer `6uwxFZAwtVKZdnqUyyA2AyqJAbWuiqAbeiGrEWvjqbFq`) |
| Test SPL mint | `4ZnWfakMkf8eNhu2EDH3t3SSZNwPie42kLRoBX4Jq4J5` |
| Percolator allowlist | `6Grr3eyuJVME5gDbeT6g6Q3pojQGSuSDaEmHDM7QYhzx`, magic "61VCREP" @1 |

### E2E transactions

| Step | Tx | Lamports |
|---|---|---|
| init_config (admin = disposable deployer) | [2BsTqi1ySs…](https://explorer.clawdchain.trade/tx/2BsTqi1ySsWQBiyEYtyvr4QMFGTUaNuqiVuSGgA61aWUwvMBWk8UC4xb7y5nzNwMHW4w2dg1NXVbLPcUKsKVkVq2) | 6519560 |
| admin_set_allowlist[0] percolator 6Grr3eyuJVME5gDbeT6g6Q3pojQGSuSDaEmHDM7QYhzx | [5yYWML5Q3W…](https://explorer.clawdchain.trade/tx/5yYWML5Q3WMGNR2zk9iZ5A1ofYhompcx45nv4BxJgneBM9qq4d3BMkNw754eCVC2JuwzqdAK2vsK23V5xkp5agoN) | 5000 |
| mint test Core agent asset | [3KHMyEyaaL…](https://explorer.clawdchain.trade/tx/3KHMyEyaaLu9eHTJqn9Ax9sXGiNTViqdMREApzV5e9MjNVxNbrMrTBGwDxFHPkEKNqCqAjVr4sPBpojD2PheHWcv) | 3493600 |
| pair(spl test mint) owner-direct, primary | [3HACkn4aKw…](https://explorer.clawdchain.trade/tx/3HACkn4aKwNcahmkdZyi43NtLdpJmq9o7tdLKj6BarfKnj9XZ9aVnGoUKkWwMg5g8wr5hvV5Vwx4aniPrC1X2ky3) | 3568520 |
| pair(percolator CLAWD-PERP (clawd chain wrapper)) via Core execute asset-signer | [MAq93dFtfn…](https://explorer.clawdchain.trade/tx/MAq93dFtfne2tE2igcoXif2KPUguBUogE7rkpASA2gATxaSvvsaFk64aAYkED6dTqgdbxPE4S552es2tyCcwvVy) | 2392280 |
| lock(percolator pairing) | [4pWHt5o5dv…](https://explorer.clawdchain.trade/tx/4pWHt5o5dv6jtLZJkQA9Me6zoT5vzcvsZTMu5MBq4n3tMm8TxAowViqqgmjLzecBd4qYKUPz2kjaTEG5fiGbBZFE) | 5000 |
| unpair(locked percolator pairing) rejected | simulated → PairingLocked ✓ | — |
| pair(spl) with wrong kind on a Percolator slab rejected | simulated → TargetOwnerMismatch ✓ | — |
| pair(other: registry config account) [UNVERIFIED] | [2gFyr3Riqf…](https://explorer.clawdchain.trade/tx/2gFyr3RiqfaPTefcM6qypSMrDauBpVyAfcQjfPi47ikw7mw2hyh6eNQGoQnm4GZNQkpJAbHyGtQqno1zLJBJXkQC) | 2343560 |
| unpair(other) -> rent refunded | [572vsgXJTH…](https://explorer.clawdchain.trade/tx/572vsgXJTH8pKSEmbHCTUQfQ724gkDb48J2RFVW8Kw6rJ5on598NhcHa3UFWEoDa57zXMyHYw897niBBrnNF3FJi) | -2333560 |

Final pairings:

```json
{
  "pairings": [
    {
      "kind": "spl",
      "target": "4ZnWfakMkf8eNhu2EDH3t3SSZNwPie42kLRoBX4Jq4J5",
      "resolved_mint": "4ZnWfakMkf8eNhu2EDH3t3SSZNwPie42kLRoBX4Jq4J5",
      "registry": "A5tHHrrXcYaswtqHzh7V711CVNKPt2mLRou5yGy3ADJm",
      "pairing": "APKfY894yd1gVjCeCbPr9TNvYzFA6FFWDWDEf2RNjebg",
      "cluster": "clawd-chain",
      "primary": true,
      "locked": false,
      "flags": 6,
      "viaAssetSigner": false,
      "verifiedMint": true,
      "setBy": "6ThLf4zU3HNxrPjNgr9NzSmgxXNEJaagzVUm8gQwBc4K",
      "targetProgram": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "slot": "8715471"
    },
    {
      "kind": "percolator-market",
      "target": "Ha2RECxyaKvXNA5KuoLxvJpEQGGdMmn3xBP4FdJQvSb5",
      "resolved_mint": "11111111111111111111111111111111",
      "registry": "A5tHHrrXcYaswtqHzh7V711CVNKPt2mLRou5yGy3ADJm",
      "pairing": "5hP85w8E9LPMRSRLTmCUhJJ4mn6RJUt8diXeDeJBBVs2",
      "cluster": "clawd-chain",
      "primary": false,
      "locked": true,
      "flags": 17,
      "viaAssetSigner": true,
      "verifiedMint": false,
      "setBy": "6uwxFZAwtVKZdnqUyyA2AyqJAbWuiqAbeiGrEWvjqbFq",
      "targetProgram": "6Grr3eyuJVME5gDbeT6g6Q3pojQGSuSDaEmHDM7QYhzx",
      "slot": "8715476"
    }
  ],
  "primary": "APKfY894yd1gVjCeCbPr9TNvYzFA6FFWDWDEf2RNjebg",
  "otherClosed": true
}
```
## Solana devnet: NOT deployed yet (blocked on test SOL)

The public devnet RPC faucet is closed (`requestAirdrop` → 410 "The RPC faucet is closed on this cluster. Request test SOL at https://faucet.solana.com"),
and faucet.solana.com needs a browser/GitHub/captcha step. No other free, keyless devnet faucet was reachable from the build box.
Needed: **~0.5 devnet SOL** to `6ThLf4zU3HNxrPjNgr9NzSmgxXNEJaagzVUm8gQwBc4K` (ProgramData 0.314 SOL at 2× max_len + a ~0.157 SOL buffer that is refunded + ~0.02 for the e2e).
Then:
```bash
solana program deploy program/target/deploy/percolator_agent_registry.so --program-id /tmp/par-keys/program.json \
  --keypair /tmp/par-keys/deployer.json --upgrade-authority /tmp/par-keys/deployer.json --max-len 61664 --url devnet
REGISTRY_KEYPAIR=/tmp/par-keys/deployer.json CLUSTER=devnet npx tsx client/scripts/e2e.ts
```

### Devnet-state fork (local validator) — passed
`solana-test-validator` cloned from devnet: Core `CoREEN…` (upgradeable), CLAWD10K-PERP slab `7KgTRxE14Rii5kR7YpPuCfKKQSceXoZNorhMtunRJDy3` (owner `Anw121…`) and CLAWD-dev mint;
registry loaded at the same program ID with the disposable upgrade authority. Same e2e script, all steps passed
(`client/runs/e2e-devnet-fork-2026-11-11T205342.json`): init_config, allowlist `Anw121…` ("PERCOLAT" @0), mint test Core asset, create test SPL mint, pair SPL (owner-direct, primary),
pair the devnet Percolator slab through Core Execute (asset-signer PDA, VIA_ASSET_SIGNER flag set), lock, locked-unpair rejected (10), wrong-kind rejected (6), pair Other + unpair refund.

## Mainnet: NOT deployed (needs dev's explicit yes, upgrade-key plan and audit)
Rent figures come from live mainnet `getMinimumBalanceForRentExemption` for the real 30,832-byte binary (2026-11-11):
ProgramData at 2× headroom (61,709 B) = 0.31413196 SOL. At 1× (30,877 B) = 0.1575054 SOL. Program account = 0.00083312. Config = 0.00475488. Writes ≈ 0.00016.
**Locked total ≈ 0.3199 SOL (2×) or ≈ 0.164 SOL (1×)**, plus ≈ 0.157 SOL that the deploy buffer holds briefly and then refunds. Each pairing is 0.00170688 SOL; a primary pointer is 0.00089408.
