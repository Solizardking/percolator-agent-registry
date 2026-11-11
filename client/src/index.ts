/**
 * Percolator Agent Registry: TypeScript client.
 * Open agent <-> token/market pairing for Metaplex Core agent assets. $CLAWD ecosystem; no new tickers.
 * UNAUDITED. Deployed on Solana devnet / clawd chain only.
 */
import { PublicKey, TransactionInstruction, SystemProgram, AccountMeta } from "@solana/web3.js";

export const CORE_PROGRAM_ID = new PublicKey("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
export const BPF_UPGRADEABLE_LOADER = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");
/** Test deployments (devnet / clawd chain). Upgrade authority is a disposable test key. */
export const REGISTRY_PROGRAM_ID_TEST = new PublicKey("A5tHHrrXcYaswtqHzh7V711CVNKPt2mLRou5yGy3ADJm");
/** $CLAWD: the only token dev's own agents pair with (mainnet Token-2022 mint). */
export const CLAWD_MINT = new PublicKey("8cHzQHUS2s2h8TzCmfqPKYiM4dSt4roa3n7MyRLApump");

export enum Kind { Spl = 0, Token2022 = 1, PumpCurve = 2, PumpSwap = 3, Raydium = 4, Meteora = 5, Genesis = 6, Percolator = 7, Perp = 8, Other = 9 }
export const KIND_NAMES = ["spl", "token2022", "pump-curve", "pumpswap", "raydium", "meteora", "genesis", "percolator-market", "perp", "other"] as const;

export const Flags = { LOCKED: 1, PRIMARY: 2, VERIFIED_MINT: 4, UNVERIFIED_TARGET: 8, VIA_ASSET_SIGNER: 16 } as const;
export const Opts = { LOCK: 1, PRIMARY: 2 } as const;
export const SIZES = { config: 808, pairing: 208, primary: 48 } as const;

export const ERRORS: Record<number, string> = {
  0: "InvalidInstruction", 1: "NotEnoughAccounts", 2: "InvalidAsset", 3: "Unauthorized", 4: "InvalidPda", 5: "InvalidKind",
  6: "TargetOwnerMismatch", 7: "TargetLayoutInvalid", 8: "ResolvedMintMismatch", 9: "ResolvedMintRequired", 10: "PairingLocked",
  11: "PrimaryLocked", 12: "AlreadyInitialized", 13: "NotInitialized", 14: "Paused", 15: "NotAdmin", 16: "InvalidProgramData",
  17: "NotUpgradeAuthority", 18: "AllowlistIndex", 19: "WrongPairing", 20: "TargetMissing",
};

const enc = (s: string) => new TextEncoder().encode(s);

export const configPda = (pid = REGISTRY_PROGRAM_ID_TEST) => PublicKey.findProgramAddressSync([enc("config")], pid)[0];
export const pairingPda = (asset: PublicKey, target: PublicKey, pid = REGISTRY_PROGRAM_ID_TEST) =>
  PublicKey.findProgramAddressSync([enc("pairing"), asset.toBuffer(), target.toBuffer()], pid)[0];
export const primaryPda = (asset: PublicKey, pid = REGISTRY_PROGRAM_ID_TEST) =>
  PublicKey.findProgramAddressSync([enc("primary"), asset.toBuffer()], pid)[0];
export const assetSignerPda = (asset: PublicKey) =>
  PublicKey.findProgramAddressSync([enc("mpl-core-execute"), asset.toBuffer()], CORE_PROGRAM_ID)[0];
export const programDataPda = (pid = REGISTRY_PROGRAM_ID_TEST) =>
  PublicKey.findProgramAddressSync([pid.toBuffer()], BPF_UPGRADEABLE_LOADER)[0];

const ix = (pid: PublicKey, keys: AccountMeta[], data: Uint8Array) => new TransactionInstruction({ programId: pid, keys, data: Buffer.from(data) });
const w = (pubkey: PublicKey, isSigner = false): AccountMeta => ({ pubkey, isSigner, isWritable: true });
const r = (pubkey: PublicKey, isSigner = false): AccountMeta => ({ pubkey, isSigner, isWritable: false });

export function initConfigIx(p: { payer: PublicKey; upgradeAuthority: PublicKey; admin: PublicKey; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  return ix(pid, [w(configPda(pid)), w(p.payer, true), r(p.upgradeAuthority, true), r(programDataPda(pid)), r(SystemProgram.programId)],
    Uint8Array.from([0, ...p.admin.toBytes()]));
}

export interface PairArgs {
  asset: PublicKey; target: PublicKey; kind: Kind;
  /** asset owner (direct) or assetSignerPda(asset) when wrapped in Core Execute */
  authority: PublicKey; payer: PublicKey;
  lock?: boolean; primary?: boolean; resolvedMint?: PublicKey;
  /** required when primary=true and a different pairing is currently primary */
  currentPrimaryPairing?: PublicKey; programId?: PublicKey;
}

export function pairIx(a: PairArgs) {
  const pid = a.programId ?? REGISTRY_PROGRAM_ID_TEST;
  const opts = (a.lock ? Opts.LOCK : 0) | (a.primary ? Opts.PRIMARY : 0);
  const data = new Uint8Array(35);
  data[0] = 1; data[1] = a.kind; data[2] = opts;
  if (a.resolvedMint) data.set(a.resolvedMint.toBytes(), 3);
  const keys = [r(configPda(pid)), r(a.asset), r(a.target), w(pairingPda(a.asset, a.target, pid)), w(a.payer, true), r(a.authority, true), r(SystemProgram.programId)];
  if (a.primary) {
    keys.push(w(primaryPda(a.asset, pid)));
    if (a.currentPrimaryPairing) keys.push(w(a.currentPrimaryPairing));
  }
  return ix(pid, keys, data);
}

export function unpairIx(p: { asset: PublicKey; target: PublicKey; authority: PublicKey; recipient: PublicKey; isPrimary?: boolean; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  const keys = [w(pairingPda(p.asset, p.target, pid)), r(p.asset), r(p.authority, true), w(p.recipient)];
  if (p.isPrimary) keys.push(w(primaryPda(p.asset, pid)));
  return ix(pid, keys, Uint8Array.from([2]));
}

export function lockIx(p: { asset: PublicKey; target: PublicKey; authority: PublicKey; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  return ix(pid, [w(pairingPda(p.asset, p.target, pid)), r(p.asset), r(p.authority, true)], Uint8Array.from([3]));
}

export function setPrimaryIx(p: { asset: PublicKey; target: PublicKey; authority: PublicKey; payer: PublicKey; currentPrimaryPairing?: PublicKey; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  const keys = [w(pairingPda(p.asset, p.target, pid)), r(p.asset), r(p.authority, true), w(primaryPda(p.asset, pid)), w(p.payer, true), r(SystemProgram.programId)];
  if (p.currentPrimaryPairing) keys.push(w(p.currentPrimaryPairing));
  return ix(pid, keys, Uint8Array.from([4]));
}

export function adminSetAllowlistIx(p: { admin: PublicKey; index: number; kind: Kind.Raydium | Kind.Meteora | Kind.Percolator | Kind.Perp; program: PublicKey; magicOffset?: number; magic?: Uint8Array; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  const magic = p.magic ?? new Uint8Array();
  if (magic.length > 8) throw new Error("magic <= 8 bytes");
  const d = new Uint8Array(45);
  d[0] = 5; d[1] = p.index; d[2] = p.kind; d[3] = p.magicOffset ?? 0; d[4] = magic.length;
  d.set(magic, 5); d.set(p.program.toBytes(), 13);
  return ix(pid, [w(configPda(pid)), r(p.admin, true)], d);
}

export function adminUpdateIx(p: { admin: PublicKey; newAdmin: PublicKey; paused: boolean; programId?: PublicKey }) {
  const pid = p.programId ?? REGISTRY_PROGRAM_ID_TEST;
  return ix(pid, [w(configPda(pid)), r(p.admin, true)], Uint8Array.from([6, ...p.newAdmin.toBytes(), p.paused ? 1 : 0]));
}

/**
 * Wrap a registry instruction in Metaplex Core `ExecuteV1` (discriminator 31) so the
 * asset-signer PDA authorises it. Encoded from Core's public instruction layout.
 */
export function coreExecuteIx(p: { asset: PublicKey; payer: PublicKey; authority: PublicKey; inner: TransactionInstruction; collection?: PublicKey }) {
  const signer = assetSignerPda(p.asset);
  const data = new Uint8Array(5 + p.inner.data.length);
  data[0] = 31;
  new DataView(data.buffer).setUint32(1, p.inner.data.length, true);
  data.set(p.inner.data, 5);
  const keys: AccountMeta[] = [
    w(p.asset), p.collection ? w(p.collection) : r(CORE_PROGRAM_ID), r(signer), w(p.payer, true), r(p.authority, true),
    r(SystemProgram.programId), r(p.inner.programId),
    ...p.inner.keys.map(k => (k.pubkey.equals(signer) ? { ...k, isSigner: false } : k)),
  ];
  return ix(CORE_PROGRAM_ID, keys, data);
}

export interface Pairing {
  address?: PublicKey; kind: Kind; kindName: string; flags: number;
  locked: boolean; primary: boolean; verifiedMint: boolean; unverifiedTarget: boolean; viaAssetSigner: boolean;
  asset: PublicKey; target: PublicKey; targetProgram: PublicKey; resolvedMint: PublicKey; setBy: PublicKey; slot: bigint;
}

export function decodePairing(data: Uint8Array, address?: PublicKey): Pairing {
  if (data.length !== SIZES.pairing || data[0] !== 2) throw new Error("not a PairingV1 account");
  const pk = (o: number) => new PublicKey(data.slice(o, o + 32));
  const flags = data[3];
  return {
    address, kind: data[2], kindName: KIND_NAMES[data[2]] ?? "unknown", flags,
    locked: !!(flags & Flags.LOCKED), primary: !!(flags & Flags.PRIMARY), verifiedMint: !!(flags & Flags.VERIFIED_MINT),
    unverifiedTarget: !!(flags & Flags.UNVERIFIED_TARGET), viaAssetSigner: !!(flags & Flags.VIA_ASSET_SIGNER),
    asset: pk(8), target: pk(40), targetProgram: pk(72), resolvedMint: pk(104), setBy: pk(136),
    slot: new DataView(data.buffer, data.byteOffset).getBigUint64(168, true),
  };
}

export function decodePrimary(data: Uint8Array) {
  if (data.length !== SIZES.primary || data[0] !== 3) throw new Error("not a PrimaryPointer account");
  return { pairing: new PublicKey(data.slice(8, 40)), slot: new DataView(data.buffer, data.byteOffset).getBigUint64(40, true) };
}

export function decodeConfig(data: Uint8Array) {
  if (data.length !== SIZES.config || data[0] !== 1) throw new Error("not a RegistryConfig account");
  const allowlist = [] as { index: number; program: PublicKey; kind: Kind; magicOffset: number; magic: Uint8Array }[];
  for (let i = 0; i < 16; i++) {
    const o = 40 + i * 48;
    const prog = data.slice(o, o + 32);
    if (prog.every(b => b === 0)) continue;
    allowlist.push({ index: i, program: new PublicKey(prog), kind: data[o + 32], magicOffset: data[o + 33], magic: data.slice(o + 40, o + 40 + data[o + 34]) });
  }
  return { version: data[2], paused: data[3] !== 0, admin: new PublicKey(data.slice(8, 40)), allowlist };
}

/** Registration-JSON entry (ERC-8004 style `tokens[]`) for an on-chain pairing. */
export function registrationTokenEntry(p: Pairing, programId = REGISTRY_PROGRAM_ID_TEST, cluster = "solana-devnet") {
  return { kind: p.kindName, target: p.target.toBase58(), resolved_mint: p.resolvedMint.toBase58(), registry: programId.toBase58(), pairing: p.address?.toBase58(), cluster, primary: p.primary, locked: p.locked };
}

/** Fetch every pairing for an asset (getProgramAccounts with a memcmp on the asset field). */
export async function fetchPairingsForAsset(conn: { getProgramAccounts: Function }, asset: PublicKey, programId = REGISTRY_PROGRAM_ID_TEST): Promise<Pairing[]> {
  const res = await conn.getProgramAccounts(programId, { filters: [{ dataSize: SIZES.pairing }, { memcmp: { offset: 8, bytes: asset.toBase58() } }] });
  return res.map((x: any) => decodePairing(x.account.data, x.pubkey));
}
