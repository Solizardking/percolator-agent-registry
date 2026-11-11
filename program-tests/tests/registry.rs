//! LiteSVM tests for percolator_agent_registry (loads ../program/target/deploy/*.so).
//! The Core program binary is NOT vendored; `scripts/fetch-fixtures.sh` dumps it from devnet.
use litesvm::LiteSVM;
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

const SO: &str = "../program/target/deploy/percolator_agent_registry.so";
const FIX: &str = "tests/fixtures";

fn a(s: &str) -> Address { Address::from_str(s).unwrap() }
fn core() -> Address { a("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d") }
fn loader() -> Address { a("BPFLoaderUpgradeab1e11111111111111111111111") }
fn token() -> Address { a("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }
fn token22() -> Address { a("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") }
fn pump() -> Address { a("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") }
fn pumpswap() -> Address { a("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") }
fn genesis() -> Address { a("GNS1S5J5AspKXgpjz6SvKL66kPaKWAhaGRhCqPRxii2B") }
fn perc_devnet() -> Address { a("Anw121Fk3gKvUHXk938cY2xVb9Eq46WX3k15WESS49PA") }
fn system() -> Address { Address::default() }
const CLAWD: &str = "8cHzQHUS2s2h8TzCmfqPKYiM4dSt4roa3n7MyRLApump";

// error codes
const E_UNAUTH: u32 = 3;
const E_OWNER: u32 = 6;
const E_LAYOUT: u32 = 7;
const E_MINT_MISMATCH: u32 = 8;
const E_MINT_REQUIRED: u32 = 9;
const E_LOCKED: u32 = 10;
const E_PRIMARY_LOCKED: u32 = 11;
const E_ALREADY: u32 = 12;
const E_PAUSED: u32 = 14;
const E_NOT_ADMIN: u32 = 15;
const E_NOT_UA: u32 = 17;

struct Env {
    svm: LiteSVM,
    pid: Address,
    ua: Keypair,
    payer: Keypair,
    owner: Keypair,
    asset: Address,
}

fn pda(seeds: &[&[u8]], p: &Address) -> Address { Address::find_program_address(seeds, p).0 }
fn config_pda(pid: &Address) -> Address { pda(&[b"config"], pid) }
fn pairing_pda(pid: &Address, asset: &Address, target: &Address) -> Address {
    pda(&[b"pairing", asset.as_ref(), target.as_ref()], pid)
}
fn primary_pda(pid: &Address, asset: &Address) -> Address { pda(&[b"primary", asset.as_ref()], pid) }

fn fake_asset(owner: &Address) -> Account {
    // Core AssetV1 bytes: key=1 | owner | update_authority (Address variant=1, key) | name "" | uri "" | seq None
    let mut d = vec![1u8];
    d.extend_from_slice(owner.as_ref());
    d.push(1);
    d.extend_from_slice(owner.as_ref());
    d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
    Account { lamports: 10_000_000, data: d, owner: core(), executable: false, rent_epoch: 0 }
}

fn acct(owner: Address, data: Vec<u8>) -> Account {
    Account { lamports: 5_000_000, data, owner, executable: false, rent_epoch: 0 }
}

fn spl_mint() -> Vec<u8> { let mut d = vec![0u8; 82]; d[45] = 1; d }

fn setup() -> Env {
    let mut svm = LiteSVM::new();
    let pid = Address::new_unique();
    let so = std::fs::read(SO).expect("build the program first: cargo build-sbf in ../program");
    svm.add_program(pid, &so).unwrap();
    let ua = Keypair::new();
    // Rewrite ProgramData so the upgrade authority is `ua` (real loader-v3 layout).
    let pd = pda(&[pid.as_ref()], &loader());
    let mut d = vec![3u8, 0, 0, 0];
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(1);
    d.extend_from_slice(ua.pubkey().as_ref());
    d.extend_from_slice(&so);
    let lam = svm.minimum_balance_for_rent_exemption(d.len());
    svm.set_account(pd, Account { lamports: lam, data: d, owner: loader(), executable: false, rent_epoch: 0 }).unwrap();

    let payer = Keypair::new();
    let owner = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&ua.pubkey(), 1_000_000_000).unwrap();
    let asset = Address::new_unique();
    svm.set_account(asset, fake_asset(&owner.pubkey())).unwrap();
    let mut e = Env { svm, pid, ua, payer, owner, asset };
    let ix = init_config_ix(&e, &e.ua.pubkey(), &e.ua.pubkey());
    let (p, u) = (e.payer.insecure_clone(), e.ua.insecure_clone());
    send(&mut e, &[ix], &[&p, &u]).unwrap();
    e
}

fn send(e: &mut Env, ixs: &[Instruction], signers: &[&Keypair]) -> Result<u64, u32> {
    e.svm.expire_blockhash();
    let msg = Message::new(ixs, Some(&signers[0].pubkey()));
    let tx = Transaction::new(signers, msg, e.svm.latest_blockhash());
    match e.svm.send_transaction(tx) {
        Ok(m) => Ok(m.compute_units_consumed),
        Err(f) => {
            let s = format!("{:?}", f.err);
            // InstructionError(_, Custom(n))
            if let Some(i) = s.find("Custom(") {
                let rest = &s[i + 7..];
                let n: u32 = rest[..rest.find(')').unwrap()].parse().unwrap();
                Err(n)
            } else {
                eprintln!("non-custom error: {s}\n{}", f.meta.logs.join("\n"));
                Err(u32::MAX)
            }
        }
    }
}

fn init_config_ix(e: &Env, ua: &Address, admin: &Address) -> Instruction {
    let mut data = vec![0u8];
    data.extend_from_slice(admin.as_ref());
    Instruction::new_with_bytes(e.pid, &data, vec![
        AccountMeta::new(config_pda(&e.pid), false),
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(*ua, true),
        AccountMeta::new_readonly(pda(&[e.pid.as_ref()], &loader()), false),
        AccountMeta::new_readonly(system(), false),
    ])
}

fn pair_ix(e: &Env, asset: &Address, target: &Address, authority: &Address, kind: u8, opts: u8,
           hint: Option<Address>, current_primary: Option<Address>) -> Instruction {
    let mut data = vec![1u8, kind, opts];
    data.extend_from_slice(hint.unwrap_or_default().as_ref());
    let mut metas = vec![
        AccountMeta::new_readonly(config_pda(&e.pid), false),
        AccountMeta::new_readonly(*asset, false),
        AccountMeta::new_readonly(*target, false),
        AccountMeta::new(pairing_pda(&e.pid, asset, target), false),
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system(), false),
    ];
    if opts & 2 != 0 {
        metas.push(AccountMeta::new(primary_pda(&e.pid, asset), false));
        if let Some(c) = current_primary { metas.push(AccountMeta::new(c, false)); }
    }
    Instruction::new_with_bytes(e.pid, &data, metas)
}

fn simple_ix(e: &Env, tag: u8, metas: Vec<AccountMeta>, data: &[u8]) -> Instruction {
    let mut d = vec![tag];
    d.extend_from_slice(data);
    Instruction::new_with_bytes(e.pid, &d, metas)
}

fn pair_owner(e: &mut Env, target: Address, kind: u8, opts: u8, hint: Option<Address>) -> Result<u64, u32> {
    let ix = pair_ix(e, &e.asset.clone(), &target, &e.owner.pubkey(), kind, opts, hint, None);
    let (p, o) = (e.payer.insecure_clone(), e.owner.insecure_clone());
    send(e, &[ix], &[&p, &o])
}

fn record(e: &Env, target: &Address) -> Vec<u8> {
    e.svm.get_account(&pairing_pda(&e.pid, &e.asset, target)).expect("pairing exists").data
}

fn allowlist(e: &mut Env, idx: u8, kind: u8, program: &Address, moff: u8, magic: &[u8]) -> Result<u64, u32> {
    let mut d = vec![idx, kind, moff, magic.len() as u8];
    let mut m = [0u8; 8];
    m[..magic.len()].copy_from_slice(magic);
    d.extend_from_slice(&m);
    d.extend_from_slice(program.as_ref());
    let ix = simple_ix(e, 5, vec![AccountMeta::new(config_pda(&e.pid), false), AccountMeta::new_readonly(e.ua.pubkey(), true)], &d);
    let (p, u) = (e.payer.insecure_clone(), e.ua.insecure_clone());
    send(e, &[ix], &[&p, &u])
}

// ---------------------------------------------------------------------------------------------

#[test]
fn init_config_requires_upgrade_authority_and_is_one_shot() {
    let mut e = setup();
    let cfg = e.svm.get_account(&config_pda(&e.pid)).unwrap();
    assert_eq!(cfg.data.len(), 808);
    assert_eq!(cfg.data[0], 1);
    assert_eq!(&cfg.data[8..40], e.ua.pubkey().as_ref());
    // second init fails
    let ix = init_config_ix(&e, &e.ua.pubkey(), &e.ua.pubkey());
    let (p, u) = (e.payer.insecure_clone(), e.ua.insecure_clone());
    assert_eq!(send(&mut e, &[ix], &[&p, &u]), Err(E_ALREADY));

    // fresh program: a stranger cannot init
    let mut e2 = setup_uninitialised();
    let stranger = Keypair::new();
    e2.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let ix = init_config_ix(&e2, &stranger.pubkey(), &stranger.pubkey());
    let p = e2.payer.insecure_clone();
    assert_eq!(send(&mut e2, &[ix], &[&p, &stranger]), Err(E_NOT_UA));
}

fn setup_uninitialised() -> Env {
    let mut svm = LiteSVM::new();
    let pid = Address::new_unique();
    let so = std::fs::read(SO).unwrap();
    svm.add_program(pid, &so).unwrap();
    let ua = Keypair::new();
    let pd = pda(&[pid.as_ref()], &loader());
    let mut d = vec![3u8, 0, 0, 0];
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(1);
    d.extend_from_slice(ua.pubkey().as_ref());
    d.extend_from_slice(&so);
    let lam = svm.minimum_balance_for_rent_exemption(d.len());
    svm.set_account(pd, Account { lamports: lam, data: d, owner: loader(), executable: false, rent_epoch: 0 }).unwrap();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    Env { svm, pid, ua, payer, owner: Keypair::new(), asset: Address::new_unique() }
}

#[test]
fn pair_spl_mint_writes_208_byte_record() {
    let mut e = setup();
    let mint = Address::new_unique();
    e.svm.set_account(mint, acct(token(), spl_mint())).unwrap();
    let cu = pair_owner(&mut e, mint, 0, 0, None).unwrap();
    let r = record(&e, &mint);
    assert_eq!(r.len(), 208);
    assert_eq!(r[0], 2); // key
    assert_eq!(r[2], 0); // kind spl
    assert_eq!(r[3], 1 << 2); // VERIFIED_MINT
    assert_eq!(&r[8..40], e.asset.as_ref());
    assert_eq!(&r[40..72], mint.as_ref());
    assert_eq!(&r[72..104], token().as_ref());
    assert_eq!(&r[104..136], mint.as_ref());
    assert_eq!(&r[136..168], e.owner.pubkey().as_ref());
    let rent = e.svm.get_account(&pairing_pda(&e.pid, &e.asset, &mint)).unwrap().lamports;
    assert_eq!(rent, e.svm.minimum_balance_for_rent_exemption(208));
    println!("pair(spl) CU = {cu}, rent = {rent} lamports");
    // double pair rejected
    assert_eq!(pair_owner(&mut e, mint, 0, 0, None), Err(E_ALREADY));
}

#[test]
fn pair_token2022_clawd_mint_fixture() {
    let mut e = setup();
    let clawd = a(CLAWD);
    let bytes = std::fs::read(format!("{FIX}/clawd_mint_t22.bin")).unwrap();
    e.svm.set_account(clawd, acct(token22(), bytes)).unwrap();
    pair_owner(&mut e, clawd, 1, 0, None).unwrap();
    let r = record(&e, &clawd);
    assert_eq!(r[2], 1);
    assert_eq!(&r[104..136], clawd.as_ref());
    // wrong kind for a token-2022 mint
    let e2_mint = Address::new_unique();
    e.svm.set_account(e2_mint, acct(token22(), spl_mint())).unwrap();
    assert_eq!(pair_owner(&mut e, e2_mint, 0, 0, None), Err(E_OWNER));
    // token account (not a mint) rejected
    let mut ta = vec![0u8; 170]; ta[45] = 1; ta[165] = 2;
    let t = Address::new_unique();
    e.svm.set_account(t, acct(token22(), ta)).unwrap();
    assert_eq!(pair_owner(&mut e, t, 1, 0, None), Err(E_LAYOUT));
}

#[test]
fn pair_pump_curve_needs_matching_mint() {
    let mut e = setup();
    let mint = Address::new_unique();
    let curve = pda(&[b"bonding-curve", mint.as_ref()], &pump());
    e.svm.set_account(curve, acct(pump(), vec![0u8; 151])).unwrap();
    assert_eq!(pair_owner(&mut e, curve, 2, 0, None), Err(E_MINT_REQUIRED));
    assert_eq!(pair_owner(&mut e, curve, 2, 0, Some(Address::new_unique())), Err(E_MINT_MISMATCH));
    pair_owner(&mut e, curve, 2, 0, Some(mint)).unwrap();
    let r = record(&e, &curve);
    assert_eq!(&r[104..136], mint.as_ref());
    assert_eq!(r[3] & 4, 4);
}

#[test]
fn pair_pumpswap_clawd_pool_resolves_base_mint() {
    let mut e = setup();
    let pool = a("GBzd5ooMFkYtYgzUMKfzyaEJpvzxvm5FK4FDJcpVJJcH");
    let bytes = std::fs::read(format!("{FIX}/clawd_pumpswap_pool.bin")).unwrap();
    e.svm.set_account(pool, acct(pumpswap(), bytes)).unwrap();
    assert_eq!(pair_owner(&mut e, pool, 3, 0, Some(Address::new_unique())), Err(E_MINT_MISMATCH));
    pair_owner(&mut e, pool, 3, 0, None).unwrap();
    let r = record(&e, &pool);
    assert_eq!(&r[104..136], a(CLAWD).as_ref());
    assert_eq!(&r[72..104], pumpswap().as_ref());
}

#[test]
fn pair_genesis_checks_layout() {
    let mut e = setup();
    let base = Address::new_unique();
    let mut d = vec![0u8; 200];
    d[0] = 18;
    d[40..72].copy_from_slice(base.as_ref());
    let g = Address::new_unique();
    e.svm.set_account(g, acct(genesis(), d.clone())).unwrap();
    let mut bad = d.clone(); bad[128] = 1;
    let g2 = Address::new_unique();
    e.svm.set_account(g2, acct(genesis(), bad)).unwrap();
    assert_eq!(pair_owner(&mut e, g2, 6, 0, None), Err(E_LAYOUT));
    pair_owner(&mut e, g, 6, 0, None).unwrap();
    assert_eq!(&record(&e, &g)[104..136], base.as_ref());
}

#[test]
fn pair_raydium_meteora_phoenix_and_other() {
    let mut e = setup();
    let ray = Address::new_unique();
    e.svm.set_account(ray, acct(a("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"), vec![0u8; 637])).unwrap();
    pair_owner(&mut e, ray, 4, 0, None).unwrap();
    let met = Address::new_unique();
    e.svm.set_account(met, acct(a("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"), vec![0u8; 900])).unwrap();
    pair_owner(&mut e, met, 5, 0, None).unwrap();
    let phx = Address::new_unique();
    e.svm.set_account(phx, acct(a("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY"), vec![0u8; 900])).unwrap();
    pair_owner(&mut e, phx, 8, 0, None).unwrap();
    // spoofed owner for raydium kind
    let fake = Address::new_unique();
    e.svm.set_account(fake, acct(Address::new_unique(), vec![0u8; 637])).unwrap();
    assert_eq!(pair_owner(&mut e, fake, 4, 0, None), Err(E_OWNER));
    // Other: accepted but flagged UNVERIFIED
    pair_owner(&mut e, fake, 9, 0, None).unwrap();
    assert_eq!(record(&e, &fake)[3] & 8, 8);
}

#[test]
fn percolator_market_needs_allowlist_and_magic() {
    let mut e = setup();
    let slab = a("7KgTRxE14Rii5kR7YpPuCfKKQSceXoZNorhMtunRJDy3");
    let bytes = std::fs::read(format!("{FIX}/devnet_clawd10k_slab.bin")).unwrap();
    e.svm.set_account(slab, acct(perc_devnet(), bytes)).unwrap();
    assert_eq!(pair_owner(&mut e, slab, 7, 0, None), Err(E_OWNER));
    // wrong magic
    allowlist(&mut e, 0, 7, &perc_devnet(), 0, b"NOTMAGIC").unwrap();
    assert_eq!(pair_owner(&mut e, slab, 7, 0, None), Err(E_OWNER));
    // real magic: u64 LE 0x504552434F4C4154 ("PERCOLAT")
    allowlist(&mut e, 0, 7, &perc_devnet(), 0, &0x504552434F4C4154u64.to_le_bytes()).unwrap();
    pair_owner(&mut e, slab, 7, 0, Some(a("CTjhnHRb85g9m3LwZpfPKkoteKeFZcuX7VHfKeCVFxmW"))).unwrap();
    let r = record(&e, &slab);
    assert_eq!(r[2], 7);
    assert_eq!(&r[72..104], perc_devnet().as_ref());
    // non-admin cannot edit allowlist
    let stranger = Keypair::new();
    e.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let mut d = vec![1u8, 7, 0, 0];
    d.extend_from_slice(&[0u8; 8]);
    d.extend_from_slice(Address::new_unique().as_ref());
    let ix = simple_ix(&e, 5, vec![AccountMeta::new(config_pda(&e.pid), false), AccountMeta::new_readonly(stranger.pubkey(), true)], &d);
    let p = e.payer.insecure_clone();
    assert_eq!(send(&mut e, &[ix], &[&p, &stranger]), Err(E_NOT_ADMIN));
}

#[test]
fn only_asset_owner_can_pair_and_ownership_follows_transfer() {
    let mut e = setup();
    let mint = Address::new_unique();
    e.svm.set_account(mint, acct(token(), spl_mint())).unwrap();
    let stranger = Keypair::new();
    e.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let ix = pair_ix(&e, &e.asset, &mint, &stranger.pubkey(), 0, 0, None, None);
    let p = e.payer.insecure_clone();
    assert_eq!(send(&mut e, &[ix], &[&p, &stranger]), Err(E_UNAUTH));
    // asset not owned by Core
    let bogus = Address::new_unique();
    let mut fa = fake_asset(&e.owner.pubkey()); fa.owner = Address::new_unique();
    e.svm.set_account(bogus, fa).unwrap();
    let ix = pair_ix(&e, &bogus, &mint, &e.owner.pubkey(), 0, 0, None, None);
    let o = e.owner.insecure_clone();
    assert_eq!(send(&mut e, &[ix], &[&p, &o]), Err(2));
    // owner pairs; after "transfer" the old owner can no longer unpair
    pair_owner(&mut e, mint, 0, 0, None).unwrap();
    e.svm.set_account(e.asset, fake_asset(&stranger.pubkey())).unwrap();
    let ix = unpair_ix(&e, &mint, &e.owner.pubkey(), None);
    assert_eq!(send(&mut e, &[ix], &[&p, &o]), Err(E_UNAUTH));
    let ix = unpair_ix(&e, &mint, &stranger.pubkey(), None);
    send(&mut e, &[ix], &[&p, &stranger]).unwrap();
}

fn unpair_ix(e: &Env, target: &Address, authority: &Address, primary: Option<Address>) -> Instruction {
    let mut metas = vec![
        AccountMeta::new(pairing_pda(&e.pid, &e.asset, target), false),
        AccountMeta::new_readonly(e.asset, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(e.payer.pubkey(), false),
    ];
    if let Some(p) = primary { metas.push(AccountMeta::new(p, false)); }
    simple_ix(e, 2, metas, &[])
}

fn lock_ix(e: &Env, target: &Address) -> Instruction {
    simple_ix(e, 3, vec![
        AccountMeta::new(pairing_pda(&e.pid, &e.asset, target), false),
        AccountMeta::new_readonly(e.asset, false),
        AccountMeta::new_readonly(e.owner.pubkey(), true),
    ], &[])
}

fn set_primary_ix(e: &Env, target: &Address, current: Option<Address>) -> Instruction {
    let mut metas = vec![
        AccountMeta::new(pairing_pda(&e.pid, &e.asset, target), false),
        AccountMeta::new_readonly(e.asset, false),
        AccountMeta::new_readonly(e.owner.pubkey(), true),
        AccountMeta::new(primary_pda(&e.pid, &e.asset), false),
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(system(), false),
    ];
    if let Some(c) = current { metas.push(AccountMeta::new(c, false)); }
    simple_ix(e, 4, metas, &[])
}

#[test]
fn unpair_refunds_rent_and_lock_is_permanent() {
    let mut e = setup();
    let m1 = Address::new_unique();
    let m2 = Address::new_unique();
    e.svm.set_account(m1, acct(token(), spl_mint())).unwrap();
    e.svm.set_account(m2, acct(token(), spl_mint())).unwrap();
    pair_owner(&mut e, m1, 0, 0, None).unwrap();
    pair_owner(&mut e, m2, 0, 1, None).unwrap(); // locked at creation
    let before = e.svm.get_account(&e.payer.pubkey()).unwrap().lamports;
    let (p, o) = (e.payer.insecure_clone(), e.owner.insecure_clone());
    let ix = unpair_ix(&e, &m1, &e.owner.pubkey(), None);
    send(&mut e, &[ix], &[&p, &o]).unwrap();
    let after = e.svm.get_account(&e.payer.pubkey()).unwrap().lamports;
    assert!(e.svm.get_account(&pairing_pda(&e.pid, &e.asset, &m1)).map(|a| a.lamports == 0).unwrap_or(true));
    assert_eq!(after + 10_000, before + e.svm.minimum_balance_for_rent_exemption(208));
    let ix = unpair_ix(&e, &m2, &e.owner.pubkey(), None);
    assert_eq!(send(&mut e, &[ix], &[&p, &o]), Err(E_LOCKED));
    // re-pair m1 then lock() it
    pair_owner(&mut e, m1, 0, 0, None).unwrap();
    let ix = lock_ix(&e, &m1);
    send(&mut e, &[ix], &[&p, &o]).unwrap();
    assert_eq!(record(&e, &m1)[3] & 1, 1);
    let ix = unpair_ix(&e, &m1, &e.owner.pubkey(), None);
    assert_eq!(send(&mut e, &[ix], &[&p, &o]), Err(E_LOCKED));
}

#[test]
fn primary_pointer_moves_and_freezes_when_locked() {
    let mut e = setup();
    let (m1, m2, m3) = (Address::new_unique(), Address::new_unique(), Address::new_unique());
    for m in [m1, m2, m3] { e.svm.set_account(m, acct(token(), spl_mint())).unwrap(); }
    let pp = primary_pda(&e.pid, &e.asset);
    pair_owner(&mut e, m1, 0, 2, None).unwrap();
    assert_eq!(&e.svm.get_account(&pp).unwrap().data[8..40], pairing_pda(&e.pid, &e.asset, &m1).as_ref());
    assert_eq!(e.svm.get_account(&pp).unwrap().data.len(), 48);
    // pairing m2 as primary must pass the current primary pairing
    let ix = pair_ix(&e, &e.asset, &m2, &e.owner.pubkey(), 0, 2, None, Some(pairing_pda(&e.pid, &e.asset, &m1)));
    let (p, o) = (e.payer.insecure_clone(), e.owner.insecure_clone());
    send(&mut e, &[ix], &[&p, &o]).unwrap();
    assert_eq!(record(&e, &m1)[3] & 2, 0);
    assert_eq!(record(&e, &m2)[3] & 2, 2);
    // lock m2 (primary) -> pointer frozen
    let ix = lock_ix(&e, &m2);
    send(&mut e, &[ix], &[&p, &o]).unwrap();
    let ix = set_primary_ix(&e, &m1, Some(pairing_pda(&e.pid, &e.asset, &m2)));
    assert_eq!(send(&mut e, &[ix], &[&p, &o]), Err(E_PRIMARY_LOCKED));
    // fresh asset: unpair primary closes the pointer
    let mut e = setup();
    for m in [m3] { e.svm.set_account(m, acct(token(), spl_mint())).unwrap(); }
    pair_owner(&mut e, m3, 0, 2, None).unwrap();
    let pp = primary_pda(&e.pid, &e.asset);
    let ix = unpair_ix(&e, &m3, &e.owner.pubkey(), Some(pp));
    let (p, o) = (e.payer.insecure_clone(), e.owner.insecure_clone());
    send(&mut e, &[ix], &[&p, &o]).unwrap();
    assert!(e.svm.get_account(&pp).map(|a| a.lamports == 0).unwrap_or(true));
}

#[test]
fn paused_registry_rejects_pairs() {
    let mut e = setup();
    let mut d = e.ua.pubkey().as_ref().to_vec(); d.push(1);
    let ix = simple_ix(&e, 6, vec![AccountMeta::new(config_pda(&e.pid), false), AccountMeta::new_readonly(e.ua.pubkey(), true)], &d);
    let (p, u) = (e.payer.insecure_clone(), e.ua.insecure_clone());
    send(&mut e, &[ix], &[&p, &u]).unwrap();
    let mint = Address::new_unique();
    e.svm.set_account(mint, acct(token(), spl_mint())).unwrap();
    assert_eq!(pair_owner(&mut e, mint, 0, 0, None), Err(E_PAUSED));
}

#[test]
fn prefunded_pda_cannot_block_pairing() {
    let mut e = setup();
    let mint = Address::new_unique();
    e.svm.set_account(mint, acct(token(), spl_mint())).unwrap();
    let pdk = pairing_pda(&e.pid, &e.asset, &mint);
    e.svm.set_account(pdk, Account { lamports: 1_000, data: vec![], owner: system(), executable: false, rent_epoch: 0 }).unwrap();
    pair_owner(&mut e, mint, 0, 0, None).unwrap();
    let acc = e.svm.get_account(&pdk).unwrap();
    assert_eq!(acc.lamports, e.svm.minimum_balance_for_rent_exemption(208));
    assert_eq!(acc.owner, e.pid);
}

// ----------------------------------------------------------------------- real Core: asset-signer route

fn borsh_str(v: &mut Vec<u8>, s: &str) { v.extend_from_slice(&(s.len() as u32).to_le_bytes()); v.extend_from_slice(s.as_bytes()); }

#[test]
fn asset_signer_pda_via_core_execute() {
    let core_so = format!("{FIX}/mpl_core_devnet.so");
    let Ok(core_bytes) = std::fs::read(&core_so) else {
        eprintln!("SKIP: {core_so} missing (run scripts/fetch-fixtures.sh)");
        return;
    };
    let mut e = setup();
    e.svm.add_program(core(), &core_bytes).unwrap();
    // CreateV1 (disc 0): DataState::AccountState, name, uri, plugins None
    let asset_kp = Keypair::new();
    let mut d = vec![0u8, 0u8];
    borsh_str(&mut d, "Registry Test Agent");
    borsh_str(&mut d, "https://example.invalid/agent.json");
    d.push(0);
    let create = Instruction::new_with_bytes(core(), &d, vec![
        AccountMeta::new(asset_kp.pubkey(), true),
        AccountMeta::new_readonly(core(), false), // collection: none
        AccountMeta::new_readonly(core(), false), // authority: none (payer)
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(e.owner.pubkey(), false), // owner
        AccountMeta::new_readonly(core(), false), // update authority: none
        AccountMeta::new_readonly(system(), false),
        AccountMeta::new_readonly(core(), false), // log wrapper: none
    ]);
    let p = e.payer.insecure_clone();
    let o = e.owner.insecure_clone();
    send(&mut e, &[create], &[&p, &asset_kp]).expect("core create");
    let asset = asset_kp.pubkey();
    let signer_pda = pda(&[b"mpl-core-execute", asset.as_ref()], &core());

    let mint = Address::new_unique();
    e.svm.set_account(mint, acct(token(), spl_mint())).unwrap();
    let inner = pair_ix(&e, &asset, &mint, &signer_pda, 0, 0, None, None);
    // ExecuteV1 (disc 31): asset, collection, asset_signer, payer, authority, system, program_id, ...inner accounts
    let mut data = vec![31u8];
    data.extend_from_slice(&(inner.data.len() as u32).to_le_bytes());
    data.extend_from_slice(&inner.data);
    let mut metas = vec![
        AccountMeta::new(asset, false),
        AccountMeta::new(core(), false),
        AccountMeta::new_readonly(signer_pda, false),
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(e.owner.pubkey(), true),
        AccountMeta::new_readonly(system(), false),
        AccountMeta::new_readonly(e.pid, false),
    ];
    for m in &inner.accounts {
        let mut m = m.clone();
        if m.pubkey == signer_pda { m.is_signer = false; }
        metas.push(m);
    }
    let exec = Instruction::new_with_bytes(core(), &data, metas.clone());
    let cu = send(&mut e, &[exec], &[&p, &o]).expect("execute -> pair");
    let r = e.svm.get_account(&pairing_pda(&e.pid, &asset, &mint)).unwrap().data;
    assert_eq!(r[3] & (1 << 4), 1 << 4, "VIA_ASSET_SIGNER flag");
    assert_eq!(&r[136..168], signer_pda.as_ref());
    println!("pair via Core execute CU = {cu}");

    // A non-owner cannot route through execute either.
    let stranger = Keypair::new();
    e.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let mint2 = Address::new_unique();
    e.svm.set_account(mint2, acct(token(), spl_mint())).unwrap();
    let inner2 = pair_ix(&e, &asset, &mint2, &signer_pda, 0, 0, None, None);
    let mut data2 = vec![31u8];
    data2.extend_from_slice(&(inner2.data.len() as u32).to_le_bytes());
    data2.extend_from_slice(&inner2.data);
    let mut metas2 = metas[..7].to_vec();
    metas2[4] = AccountMeta::new_readonly(stranger.pubkey(), true);
    for m in &inner2.accounts { let mut m = m.clone(); if m.pubkey == signer_pda { m.is_signer = false; } metas2.push(m); }
    let exec2 = Instruction::new_with_bytes(core(), &data2, metas2);
    assert!(send(&mut e, &[exec2], &[&p, &stranger]).is_err());
}
