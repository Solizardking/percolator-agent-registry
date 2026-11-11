import { test } from "node:test";
import assert from "node:assert/strict";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as R from "../src/index.ts";

test("pda derivations are stable", () => {
  const asset = new PublicKey("HnfZet9JoK8h3KH4GM24od4oWVBvwYjfC5noByzLx1m4");
  assert.equal(R.assetSignerPda(asset).toBase58(), "DgP6EwDE2tUbqcP8qmdtcGPeT2pjdGP13BZwaeyTYuq1");
  assert.ok(R.pairingPda(asset, R.CLAWD_MINT));
});

test("pair ix encodes kind/opts/hint and optional primary accounts", () => {
  const asset = Keypair.generate().publicKey, target = Keypair.generate().publicKey, owner = Keypair.generate().publicKey;
  const ix = R.pairIx({ asset, target, kind: R.Kind.Token2022, authority: owner, payer: owner, lock: true, primary: true, resolvedMint: target });
  assert.equal(ix.data.length, 35);
  assert.deepEqual([...ix.data.slice(0, 3)], [1, 1, 3]);
  assert.equal(ix.keys.length, 8);
  assert.ok(ix.keys[7].pubkey.equals(R.primaryPda(asset)));
});

test("core execute wrapper strips asset-signer signer flag", () => {
  const asset = Keypair.generate().publicKey, owner = Keypair.generate().publicKey, target = Keypair.generate().publicKey;
  const signer = R.assetSignerPda(asset);
  const inner = R.pairIx({ asset, target, kind: R.Kind.Spl, authority: signer, payer: owner });
  const ex = R.coreExecuteIx({ asset, payer: owner, authority: owner, inner });
  assert.equal(ex.data[0], 31);
  assert.equal(ex.data.readUInt32LE(1), 35);
  const s = ex.keys.filter(k => k.pubkey.equals(signer));
  assert.ok(s.every(k => !k.isSigner));
  assert.equal(ex.keys.length, 7 + inner.keys.length);
});

test("decodePairing round-trip", () => {
  const d = new Uint8Array(208); d[0] = 2; d[2] = 7; d[3] = R.Flags.LOCKED | R.Flags.PRIMARY;
  const a = Keypair.generate().publicKey; d.set(a.toBytes(), 8);
  new DataView(d.buffer).setBigUint64(168, 42n, true);
  const p = R.decodePairing(d);
  assert.equal(p.kindName, "percolator-market"); assert.ok(p.locked && p.primary && !p.viaAssetSigner);
  assert.ok(p.asset.equals(a)); assert.equal(p.slot, 42n);
});
