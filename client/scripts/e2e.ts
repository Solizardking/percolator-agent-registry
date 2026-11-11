/**
 * End-to-end run on Solana DEVNET or CLAWD CHAIN only (refuses any other genesis hash).
 * Uses a disposable, faucet-funded keypair given by REGISTRY_KEYPAIR (must live under /tmp).
 *   REGISTRY_KEYPAIR=/tmp/par-keys/deployer.json CLUSTER=clawd npx tsx scripts/e2e.ts
 */
import { Connection, Keypair, PublicKey, Transaction, sendAndConfirmTransaction, SystemProgram } from "@solana/web3.js";
import { createMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { readFileSync, writeFileSync } from "node:fs";
import * as R from "../src/index.ts";

const CLUSTERS: Record<string, { rpc: string; genesis: string; explorer: (s: string, k?: string) => string; perc: { program: string; slab: string; magicOffset: number; magic: number[]; resolved?: string; label: string } }> = {
  devnet: {
    rpc: process.env.RPC_URL || "https://api.devnet.solana.com",
    genesis: "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
    explorer: (s, k = "tx") => `https://explorer.solana.com/${k}/${s}?cluster=devnet`,
    perc: { program: "Anw121Fk3gKvUHXk938cY2xVb9Eq46WX3k15WESS49PA", slab: "7KgTRxE14Rii5kR7YpPuCfKKQSceXoZNorhMtunRJDy3", magicOffset: 0, magic: [...Buffer.from("TALOCREP")], resolved: "CTjhnHRb85g9m3LwZpfPKkoteKeFZcuX7VHfKeCVFxmW", label: "CLAWD10K-PERP (devnet clawd-market)" },
  },
  "devnet-fork": {
    rpc: "http://127.0.0.1:18899",
    genesis: "LOCAL",
    explorer: (s, k = "tx") => `local:${k}/${s}`,
    perc: { program: "Anw121Fk3gKvUHXk938cY2xVb9Eq46WX3k15WESS49PA", slab: "7KgTRxE14Rii5kR7YpPuCfKKQSceXoZNorhMtunRJDy3", magicOffset: 0, magic: [...Buffer.from("TALOCREP")], resolved: "CTjhnHRb85g9m3LwZpfPKkoteKeFZcuX7VHfKeCVFxmW", label: "CLAWD10K-PERP (devnet slab cloned into local validator)" },
  },
  clawd: {
    rpc: process.env.RPC_URL || "https://rpc.clawdchain.trade",
    genesis: "BjbvUdJLreJzCfVEQZHi3D4QttNyp3P5mNXEvHw56kBP",
    explorer: (s, k = "tx") => `https://explorer.clawdchain.trade/${k === "tx" ? "tx" : "address"}/${s}`,
    perc: { program: "6Grr3eyuJVME5gDbeT6g6Q3pojQGSuSDaEmHDM7QYhzx", slab: "Ha2RECxyaKvXNA5KuoLxvJpEQGGdMmn3xBP4FdJQvSb5", magicOffset: 1, magic: [...Buffer.from("61VCREP")], label: "CLAWD-PERP (clawd chain wrapper)" },
  },
};

const name = process.env.CLUSTER || "devnet";
if (/mainnet/i.test(name) || /mainnet/i.test(process.env.RPC_URL || "")) throw new Error("mainnet refused");
const C = CLUSTERS[name];
if (!C) throw new Error("CLUSTER must be devnet or clawd");
const kpPath = process.env.REGISTRY_KEYPAIR || "";
if (!kpPath.startsWith("/tmp/") || /house\.json|\.ows/.test(kpPath)) throw new Error("REGISTRY_KEYPAIR must be a disposable key under /tmp");
const payer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(kpPath, "utf8"))));
const pid = new PublicKey(process.env.PROGRAM_ID || R.REGISTRY_PROGRAM_ID_TEST.toBase58());
const conn = new Connection(C.rpc, "confirmed");
const log: any = { cluster: name, rpc: C.rpc, programId: pid.toBase58(), payer: payer.publicKey.toBase58(), steps: [] as any[] };
const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));

async function send(label: string, ixs: any[], extra: Keypair[] = []) {
  const tx = new Transaction().add(...ixs);
  const before = await conn.getBalance(payer.publicKey);
  const sig = await sendAndConfirmTransaction(conn, tx, [payer, ...extra], { commitment: "confirmed" });
  await sleep(400);
  const after = await conn.getBalance(payer.publicKey);
  const step = { label, sig, url: C.explorer(sig), lamportsSpent: before - after };
  log.steps.push(step);
  console.log(`✓ ${label}  ${sig}  (${before - after} lamports)`);
  return sig;
}

async function expectFail(label: string, ixs: any[], code: number) {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = payer.publicKey;
  tx.recentBlockhash = (await conn.getLatestBlockhash()).blockhash;
  tx.sign(payer);
  const sim = await conn.simulateTransaction(tx);
  const errStr = JSON.stringify(sim.value.err);
  const ok = errStr.includes(`"Custom":${code}`);
  log.steps.push({ label, simulated: true, expectedError: R.ERRORS[code], got: errStr, pass: ok });
  console.log(`${ok ? "✓" : "✗"} ${label} -> ${errStr}`);
  if (!ok) throw new Error(`expected ${R.ERRORS[code]}`);
}

function coreCreateIx(asset: PublicKey, owner: PublicKey, nm: string, uri: string) {
  const s = (x: string) => { const b = Buffer.from(x); const l = Buffer.alloc(4); l.writeUInt32LE(b.length); return Buffer.concat([l, b]); };
  const data = Buffer.concat([Buffer.from([0, 0]), s(nm), s(uri), Buffer.from([0])]);
  const C0 = R.CORE_PROGRAM_ID;
  return new (require_ix())({ programId: C0, data, keys: [
    { pubkey: asset, isSigner: true, isWritable: true }, { pubkey: C0, isSigner: false, isWritable: false },
    { pubkey: C0, isSigner: false, isWritable: false }, { pubkey: payer.publicKey, isSigner: true, isWritable: true },
    { pubkey: owner, isSigner: false, isWritable: false }, { pubkey: C0, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, { pubkey: C0, isSigner: false, isWritable: false },
  ] });
}
import { TransactionInstruction } from "@solana/web3.js";
const require_ix = () => TransactionInstruction;

(async () => {
  const g = await conn.getGenesisHash();
  if (C.genesis === "LOCAL") {
    if (!/^http:\/\/127\.0\.0\.1/.test(C.rpc)) throw new Error("devnet-fork must be localhost");
    if ((await conn.getBalance(payer.publicKey)) < 1e9) { const s = await conn.requestAirdrop(payer.publicKey, 5e9); await conn.confirmTransaction(s, "confirmed"); }
  } else if (g !== C.genesis) throw new Error(`genesis ${g} is not ${name}; refusing (no mainnet)`);
  log.genesis = g;
  console.log(`cluster=${name} payer=${payer.publicKey.toBase58()} balance=${(await conn.getBalance(payer.publicKey)) / 1e9}`);
  const prog = await conn.getAccountInfo(pid);
  if (!prog?.executable) throw new Error("registry program not deployed on this cluster");

  // 1. config (upgrade authority = disposable deployer)
  const cfg = await conn.getAccountInfo(R.configPda(pid));
  if (!cfg) await send("init_config (admin = disposable deployer)", [R.initConfigIx({ payer: payer.publicKey, upgradeAuthority: payer.publicKey, admin: payer.publicKey, programId: pid })]);
  // 2. allowlist this cluster's Percolator program with its slab magic
  await send(`admin_set_allowlist[0] percolator ${C.perc.program}`, [R.adminSetAllowlistIx({ admin: payer.publicKey, index: 0, kind: R.Kind.Percolator, program: new PublicKey(C.perc.program), magicOffset: C.perc.magicOffset, magic: Uint8Array.from(C.perc.magic), programId: pid })]);

  // 3. disposable test agent: Core asset owned by the disposable key
  const assetKp = Keypair.generate();
  const asset = assetKp.publicKey;
  await send("mint test Core agent asset", [coreCreateIx(asset, payer.publicKey, "Registry Test Agent", "https://arweave.net/v9XDBEfoRlQCIKLolsjJm-ywC8VB0hGVNCyf4Pgtgns")], [assetKp]);
  log.asset = asset.toBase58(); log.assetSigner = R.assetSignerPda(asset).toBase58();

  // 4. test SPL mint (no metadata, no ticker)
  const mint = await createMint(conn, payer, payer.publicKey, null, 6, undefined, { commitment: "confirmed" }, TOKEN_PROGRAM_ID);
  log.testMint = mint.toBase58();
  console.log(`✓ test SPL mint ${mint.toBase58()}`);

  // 5. pair SPL mint, owner signs directly, PRIMARY
  await send("pair(spl test mint) owner-direct, primary", [R.pairIx({ asset, target: mint, kind: R.Kind.Spl, authority: payer.publicKey, payer: payer.publicKey, primary: true, programId: pid })]);

  // 6. pair Percolator market through Core Execute (asset-signer PDA route), then lock it
  const slab = new PublicKey(C.perc.slab);
  const signer = R.assetSignerPda(asset);
  const inner = R.pairIx({ asset, target: slab, kind: R.Kind.Percolator, authority: signer, payer: payer.publicKey, programId: pid, resolvedMint: C.perc.resolved ? new PublicKey(C.perc.resolved) : undefined });
  await send(`pair(percolator ${C.perc.label}) via Core execute asset-signer`, [R.coreExecuteIx({ asset, payer: payer.publicKey, authority: payer.publicKey, inner })]);
  await send("lock(percolator pairing)", [R.lockIx({ asset, target: slab, authority: payer.publicKey, programId: pid })]);
  await expectFail("unpair(locked percolator pairing) rejected", [R.unpairIx({ asset, target: slab, authority: payer.publicKey, recipient: payer.publicKey, programId: pid })], 10);

  // 7. negative checks (simulate only)
  const stranger = Keypair.generate();
  const fake = Keypair.generate().publicKey;
  await expectFail("pair(spl) with wrong kind on a Percolator slab rejected", [R.pairIx({ asset, target: slab, kind: R.Kind.Spl, authority: payer.publicKey, payer: payer.publicKey, programId: pid })], 6);

  // 8. Other target, then unpair to show the rent refund
  const other = (await conn.getAccountInfo(R.configPda(pid))) ? R.configPda(pid) : fake; // any existing account
  await send("pair(other: registry config account) [UNVERIFIED]", [R.pairIx({ asset, target: other, kind: R.Kind.Other, authority: payer.publicKey, payer: payer.publicKey, programId: pid })]);
  await send("unpair(other) -> rent refunded", [R.unpairIx({ asset, target: other, authority: payer.publicKey, recipient: payer.publicKey, programId: pid })]);

  // 9. read back
  const read = async (t: PublicKey) => { const a = await conn.getAccountInfo(R.pairingPda(asset, t, pid)); return a ? R.decodePairing(a.data, R.pairingPda(asset, t, pid)) : null; };
  const out = [] as any[];
  for (const t of [mint, slab]) {
    const p = await read(t);
    if (!p) throw new Error("missing pairing");
    out.push({ ...R.registrationTokenEntry(p, pid, name === "clawd" ? "clawd-chain" : name === "devnet" ? "solana-devnet" : "local-devnet-fork"), flags: p.flags, viaAssetSigner: p.viaAssetSigner, verifiedMint: p.verifiedMint, setBy: p.setBy.toBase58(), targetProgram: p.targetProgram.toBase58(), slot: p.slot.toString() });
  }
  const pp = await conn.getAccountInfo(R.primaryPda(asset, pid));
  log.primary = pp ? R.decodePrimary(pp.data).pairing.toBase58() : null;
  log.pairings = out;
  log.otherClosed = !(await conn.getAccountInfo(R.pairingPda(asset, other, pid)));
  log.endBalance = await conn.getBalance(payer.publicKey);
  const file = `e2e-${name}-${new Date().toISOString().slice(0, 19).replace(/:/g, "")}.json`;
  writeFileSync(`runs/${file}`, JSON.stringify(log, null, 2));
  console.log(JSON.stringify({ pairings: out, primary: log.primary, otherClosed: log.otherClosed }, null, 2));
  console.log(`wrote runs/${file}`);
})().catch(e => { console.error("E2E FAILED:", e?.message || e); if (e?.logs) console.error(e.logs.join("\n")); process.exit(1); });
