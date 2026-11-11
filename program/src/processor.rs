use crate::error::{err, RegistryError as E};
use crate::state::*;
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::{Allocate, Assign, CreateAccount, Transfer};

pub const IX_INIT_CONFIG: u8 = 0;
pub const IX_PAIR: u8 = 1;
pub const IX_UNPAIR: u8 = 2;
pub const IX_LOCK: u8 = 3;
pub const IX_SET_PRIMARY: u8 = 4;
pub const IX_ADMIN_SET_ALLOWLIST: u8 = 5;
pub const IX_ADMIN_UPDATE: u8 = 6;

pub const OPT_LOCK: u8 = 1 << 0;
pub const OPT_PRIMARY: u8 = 1 << 1;

#[inline(never)]
pub fn process_instruction(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let (tag, rest) = data.split_first().ok_or(ProgramError::Custom(E::InvalidInstruction as u32))?;
    match *tag {
        IX_INIT_CONFIG => init_config(program_id, accounts, rest),
        IX_PAIR => pair(program_id, accounts, rest),
        IX_UNPAIR => unpair(program_id, accounts),
        IX_LOCK => lock(program_id, accounts),
        IX_SET_PRIMARY => set_primary_ix(program_id, accounts),
        IX_ADMIN_SET_ALLOWLIST => admin_set_allowlist(program_id, accounts, rest),
        IX_ADMIN_UPDATE => admin_update(program_id, accounts, rest),
        _ => err(E::InvalidInstruction),
    }
}

// ------------------------------------------------------------------ helpers

#[inline(always)]
fn need(accounts: &[AccountView], n: usize) -> ProgramResult {
    if accounts.len() < n { err(E::NotEnoughAccounts) } else { Ok(()) }
}

#[inline(always)]
fn addr32(d: &[u8], off: usize) -> Result<Address, ProgramError> {
    if d.len() < off + 32 { return err(E::InvalidInstruction); }
    Ok(read_addr(d, off))
}

/// Parse a Metaplex Core AssetV1 account by its public byte layout:
/// data[0] = Key (1 = AssetV1), data[1..33] = owner.
fn asset_owner(asset: &AccountView) -> Result<Address, ProgramError> {
    if !asset.owned_by(&CORE) { return err(E::InvalidAsset); }
    let d = asset.try_borrow()?;
    if d.len() < 66 || d[0] != 1 { return err(E::InvalidAsset); }
    Ok(read_addr(&d, 1))
}

/// Authority = asset owner (direct signer) OR the Core asset-signer PDA
/// (which can only sign through Core Execute, approved by the owner/delegate).
/// Returns true when the asset-signer route was used.
fn check_authority(asset: &AccountView, authority: &AccountView) -> Result<bool, ProgramError> {
    if !authority.is_signer() { return err(E::Unauthorized); }
    let owner = asset_owner(asset)?;
    if authority.address() == &owner { return Ok(false); }
    let (signer_pda, _) = Address::find_program_address(&[ASSET_SIGNER_SEED, asset.address().as_ref()], &CORE);
    if authority.address() == &signer_pda { return Ok(true); }
    err(E::Unauthorized)
}

fn create_pda(program_id: &Address, payer: &AccountView, acct: &AccountView, space: usize, signer: Signer) -> ProgramResult {
    let need_lamports = Rent::get()?.try_minimum_balance(space)?;
    let have = acct.lamports();
    if have == 0 {
        CreateAccount { from: payer, to: acct, lamports: need_lamports, space: space as u64, owner: program_id }
            .invoke_signed(&[signer])
    } else {
        // Pre-funded PDA (griefing-safe path): top up, allocate, assign.
        if have < need_lamports {
            Transfer { from: payer, to: acct, lamports: need_lamports - have }.invoke()?;
        }
        Allocate { account: acct, space: space as u64 }.invoke_signed(&[signer.clone()])?;
        Assign { account: acct, owner: program_id }.invoke_signed(&[signer])
    }
}

fn close_to(acct: &mut AccountView, dest: &mut AccountView) -> ProgramResult {
    let l = acct.lamports();
    dest.set_lamports(dest.lamports().checked_add(l).ok_or(ProgramError::ArithmeticOverflow)?);
    acct.set_lamports(0);
    acct.close()
}

fn load_config(program_id: &Address, cfg: &AccountView) -> ProgramResult {
    let (pda, _) = Address::find_program_address(&[CONFIG_SEED], program_id);
    if cfg.address() != &pda { return err(E::InvalidPda); }
    if !cfg.owned_by(program_id) || cfg.data_len() != CONFIG_LEN { return err(E::NotInitialized); }
    let d = cfg.try_borrow()?;
    if d[0] != KEY_CONFIG { return err(E::NotInitialized); }
    Ok(())
}

fn load_pairing(program_id: &Address, p: &AccountView, asset: &AccountView) -> ProgramResult {
    if !p.owned_by(program_id) || p.data_len() != PAIRING_LEN { return err(E::NotInitialized); }
    let d = p.try_borrow()?;
    if d[P_KEY] != KEY_PAIRING || read_addr(&d, P_ASSET) != *asset.address() { return err(E::WrongPairing); }
    Ok(())
}

// ------------------------------------------------------------------ target verification

struct Verified { program: Address, resolved: Address, flags: u8 }

fn allowlisted(cfg: &[u8], kind: Kind, owner: &Address, data: &[u8]) -> bool {
    let mut i = 0;
    while i < ALLOW_SLOTS {
        let e = &cfg[CFG_ENTRIES + i * ALLOW_ENTRY_LEN..CFG_ENTRIES + (i + 1) * ALLOW_ENTRY_LEN];
        if e[ENT_KIND] == kind as u8 && &e[..32] == owner.as_ref() && e[..32] != [0u8; 32] {
            let mlen = e[ENT_MLEN] as usize;
            if mlen == 0 { return true; }
            let moff = e[ENT_MOFF] as usize;
            if mlen <= 8 && data.len() >= moff + mlen && data[moff..moff + mlen] == e[ENT_MAGIC..ENT_MAGIC + mlen] {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn verify_target(cfg: &[u8], kind: Kind, target: &AccountView, hint: &Address) -> Result<Verified, ProgramError> {
    let owner = target.owner();
    let zero = Address::new_from_array([0u8; 32]);
    if target.lamports() == 0 { return err(E::TargetMissing); }
    let d = target.try_borrow()?;
    let mut v = Verified { program: owner, resolved: *hint, flags: 0 };
    match kind {
        Kind::Spl => {
            if owner != TOKEN { return err(E::TargetOwnerMismatch); }
            if d.len() != 82 || d[45] != 1 { return err(E::TargetLayoutInvalid); }
            v.resolved = *target.address();
            v.flags |= FLAG_VERIFIED_MINT;
        }
        Kind::Token2022 => {
            if owner != TOKEN_2022 { return err(E::TargetOwnerMismatch); }
            let is_mint = d.len() == 82 || (d.len() > 165 && d[165] == 1);
            if !is_mint || d[45] != 1 { return err(E::TargetLayoutInvalid); }
            v.resolved = *target.address();
            v.flags |= FLAG_VERIFIED_MINT;
        }
        Kind::PumpCurve => {
            if owner != PUMP { return err(E::TargetOwnerMismatch); }
            if *hint == zero { return err(E::ResolvedMintRequired); }
            let (pda, _) = Address::find_program_address(&[b"bonding-curve", hint.as_ref()], &PUMP);
            if pda != *target.address() { return err(E::ResolvedMintMismatch); }
            v.flags |= FLAG_VERIFIED_MINT;
        }
        Kind::PumpSwap => {
            if owner != PUMP_SWAP { return err(E::TargetOwnerMismatch); }
            // Pool: disc[8] | bump u8 | index u16 | creator[32] | base_mint[32]@43 | quote_mint[32]@75
            if d.len() < 107 { return err(E::TargetLayoutInvalid); }
            let base = read_addr(&d, 43);
            let quote = read_addr(&d, 75);
            if *hint == zero { v.resolved = base; }
            else if *hint != base && *hint != quote { return err(E::ResolvedMintMismatch); }
            v.flags |= FLAG_VERIFIED_MINT;
        }
        Kind::Genesis => {
            // GenesisAccountV2: disc u8 = 18 @0, base_mint @40, funding_mode u8 @128 (0 = Mint)
            if owner != GENESIS { return err(E::TargetOwnerMismatch); }
            if d.len() < 136 || d[0] != 18 || d[128] != 0 { return err(E::TargetLayoutInvalid); }
            let base = read_addr(&d, 40);
            if *hint != zero && *hint != base { return err(E::ResolvedMintMismatch); }
            v.resolved = base;
            v.flags |= FLAG_VERIFIED_MINT;
        }
        Kind::Raydium => {
            if !RAYDIUM.contains(&owner) && !allowlisted(cfg, kind, &owner, &d) { return err(E::TargetOwnerMismatch); }
        }
        Kind::Meteora => {
            if !METEORA.contains(&owner) && !allowlisted(cfg, kind, &owner, &d) { return err(E::TargetOwnerMismatch); }
        }
        Kind::Percolator => {
            if !allowlisted(cfg, kind, &owner, &d) { return err(E::TargetOwnerMismatch); }
        }
        Kind::Perp => {
            if owner != PHOENIX && !allowlisted(cfg, kind, &owner, &d) { return err(E::TargetOwnerMismatch); }
        }
        Kind::Other => {
            v.flags |= FLAG_UNVERIFIED_TARGET;
        }
    }
    Ok(v)
}

// ------------------------------------------------------------------ instructions

/// 0 InitConfig { admin: [32] }
/// accounts: config(w), payer(s,w), upgrade_authority(s), program_data, system
fn init_config(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    need(accounts, 5)?;
    let admin = addr32(data, 0)?;
    let (cfg, payer, ua, pd) = (accounts[0], accounts[1], accounts[2], accounts[3]);
    if !payer.is_signer() || !ua.is_signer() { return err(E::Unauthorized); }
    // Only the program's upgrade authority may initialise (stops front-running).
    let (pd_addr, _) = Address::find_program_address(&[program_id.as_ref()], &BPF_UPGRADEABLE_LOADER);
    if pd.address() != &pd_addr || !pd.owned_by(&BPF_UPGRADEABLE_LOADER) { return err(E::InvalidProgramData); }
    {
        let d = pd.try_borrow()?;
        if d.len() < 45 || d[0..4] != [3, 0, 0, 0] { return err(E::InvalidProgramData); }
        if d[12] != 1 || &d[13..45] != ua.address().as_ref() { return err(E::NotUpgradeAuthority); }
    }
    let (pda, bump) = Address::find_program_address(&[CONFIG_SEED], program_id);
    if cfg.address() != &pda { return err(E::InvalidPda); }
    if cfg.owned_by(program_id) { return err(E::AlreadyInitialized); }
    let b = [bump];
    let seeds = [Seed::from(CONFIG_SEED), Seed::from(&b)];
    create_pda(program_id, &payer, &cfg, CONFIG_LEN, Signer::from(&seeds))?;
    let mut cfg = cfg;
    let mut d = cfg.try_borrow_mut()?;
    d[0] = KEY_CONFIG;
    d[1] = bump;
    d[2] = VERSION;
    write_addr(&mut d, CFG_ADMIN, &admin);
    Ok(())
}

/// 1 Pair { kind u8, opts u8 (1 = lock, 2 = primary), resolved_mint [32] (zero = auto/none) }
/// accounts: config, asset, target, pairing(w), payer(s,w), authority(s), system,
///           [primary_pointer(w)], [current_primary_pairing(w)]
fn pair(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    need(accounts, 7)?;
    if data.len() < 34 { return err(E::InvalidInstruction); }
    let kind = Kind::from_u8(data[0]).ok_or(ProgramError::Custom(E::InvalidKind as u32))?;
    let opts = data[1];
    let hint = read_addr(data, 2);
    let (cfg, asset, target, pairing, payer, authority) =
        (accounts[0], accounts[1], accounts[2], accounts[3], accounts[4], accounts[5]);
    load_config(program_id, &cfg)?;
    let v = {
        let c = cfg.try_borrow()?;
        if c[CFG_PAUSED] != 0 { return err(E::Paused); }
        verify_target(&c, kind, &target, &hint)?
    };
    if !payer.is_signer() { return err(E::Unauthorized); }
    let via_signer = check_authority(&asset, &authority)?;

    let (pda, bump) = Address::find_program_address(
        &[PAIRING_SEED, asset.address().as_ref(), target.address().as_ref()], program_id);
    if pairing.address() != &pda { return err(E::InvalidPda); }
    if pairing.owned_by(program_id) { return err(E::AlreadyInitialized); }
    let b = [bump];
    let seeds = [Seed::from(PAIRING_SEED), Seed::from(asset.address().as_ref()), Seed::from(target.address().as_ref()), Seed::from(&b)];
    create_pda(program_id, &payer, &pairing, PAIRING_LEN, Signer::from(&seeds))?;

    let slot = Clock::get()?.slot;
    {
        let mut p = pairing;
        let mut d = p.try_borrow_mut()?;
        d[P_KEY] = KEY_PAIRING;
        d[P_BUMP] = bump;
        d[P_KIND] = kind as u8;
        let mut flags = v.flags;
        if via_signer { flags |= FLAG_VIA_ASSET_SIGNER; }
        if opts & OPT_LOCK != 0 { flags |= FLAG_LOCKED; }
        d[P_FLAGS] = flags;
        write_addr(&mut d, P_ASSET, asset.address());
        write_addr(&mut d, P_TARGET, target.address());
        write_addr(&mut d, P_TARGET_PROGRAM, &v.program);
        write_addr(&mut d, P_RESOLVED_MINT, &v.resolved);
        write_addr(&mut d, P_SET_BY, authority.address());
        d[P_SLOT..P_SLOT + 8].copy_from_slice(&slot.to_le_bytes());
    }
    if opts & OPT_PRIMARY != 0 {
        need(accounts, 8)?;
        let current = if accounts.len() > 8 { Some(accounts[8]) } else { None };
        set_primary(program_id, &asset, &pairing, &accounts[7], &payer, current, slot)?;
    }
    Ok(())
}

fn set_primary(program_id: &Address, asset: &AccountView, pairing: &AccountView, primary: &AccountView,
               payer: &AccountView, current: Option<AccountView>, slot: u64) -> ProgramResult {
    let (pda, bump) = Address::find_program_address(&[PRIMARY_SEED, asset.address().as_ref()], program_id);
    if primary.address() != &pda { return err(E::InvalidPda); }
    if !primary.owned_by(program_id) {
        if !payer.is_signer() { return err(E::Unauthorized); }
        let b = [bump];
        let seeds = [Seed::from(PRIMARY_SEED), Seed::from(asset.address().as_ref()), Seed::from(&b)];
        create_pda(program_id, payer, primary, PRIMARY_LEN, Signer::from(&seeds))?;
        let mut pv = *primary;
        let mut d = pv.try_borrow_mut()?;
        d[0] = KEY_PRIMARY;
        d[1] = bump;
    } else {
        let cur = {
            let d = primary.try_borrow()?;
            if d.len() != PRIMARY_LEN || d[0] != KEY_PRIMARY { return err(E::InvalidPda); }
            read_addr(&d, PP_PAIRING)
        };
        if cur != *pairing.address() && cur != Address::new_from_array([0u8; 32]) {
            let mut c = current.ok_or(ProgramError::Custom(E::WrongPairing as u32))?;
            if c.address() != &cur { return err(E::WrongPairing); }
            if c.owned_by(program_id) && c.data_len() == PAIRING_LEN {
                let mut d = c.try_borrow_mut()?;
                if d[P_KEY] == KEY_PAIRING {
                    if d[P_FLAGS] & FLAG_LOCKED != 0 && d[P_FLAGS] & FLAG_PRIMARY != 0 { return err(E::PrimaryLocked); }
                    d[P_FLAGS] &= !FLAG_PRIMARY;
                }
            }
        }
    }
    {
        let mut pv = *primary;
        let mut d = pv.try_borrow_mut()?;
        write_addr(&mut d, PP_PAIRING, pairing.address());
        d[PP_SLOT..PP_SLOT + 8].copy_from_slice(&slot.to_le_bytes());
    }
    let mut p = *pairing;
    let mut d = p.try_borrow_mut()?;
    d[P_FLAGS] |= FLAG_PRIMARY;
    Ok(())
}

/// 2 Unpair — accounts: pairing(w), asset, authority(s), recipient(w), [primary_pointer(w)]
fn unpair(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    need(accounts, 4)?;
    let (mut pairing, asset, authority, mut recipient) = (accounts[0], accounts[1], accounts[2], accounts[3]);
    load_pairing(program_id, &pairing, &asset)?;
    check_authority(&asset, &authority)?;
    let flags = pairing.try_borrow()?[P_FLAGS];
    if flags & FLAG_LOCKED != 0 { return err(E::PairingLocked); }
    if flags & FLAG_PRIMARY != 0 {
        need(accounts, 5)?;
        let mut primary = accounts[4];
        let (pda, _) = Address::find_program_address(&[PRIMARY_SEED, asset.address().as_ref()], program_id);
        if primary.address() != &pda || !primary.owned_by(program_id) { return err(E::InvalidPda); }
        let cur = read_addr(&primary.try_borrow()?, PP_PAIRING);
        if cur == *pairing.address() { close_to(&mut primary, &mut recipient)?; }
    }
    close_to(&mut pairing, &mut recipient)
}

/// 3 Lock (one-way) — accounts: pairing(w), asset, authority(s)
fn lock(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    need(accounts, 3)?;
    let (mut pairing, asset, authority) = (accounts[0], accounts[1], accounts[2]);
    load_pairing(program_id, &pairing, &asset)?;
    check_authority(&asset, &authority)?;
    pairing.try_borrow_mut()?[P_FLAGS] |= FLAG_LOCKED;
    Ok(())
}

/// 4 SetPrimary — accounts: pairing(w), asset, authority(s), primary(w), payer(s,w), system, [current(w)]
fn set_primary_ix(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    need(accounts, 6)?;
    let (pairing, asset, authority, primary, payer) = (accounts[0], accounts[1], accounts[2], accounts[3], accounts[4]);
    load_pairing(program_id, &pairing, &asset)?;
    check_authority(&asset, &authority)?;
    let current = if accounts.len() > 6 { Some(accounts[6]) } else { None };
    let slot = Clock::get()?.slot;
    set_primary(program_id, &asset, &pairing, &primary, &payer, current, slot)
}

fn check_admin(program_id: &Address, cfg: &AccountView, admin: &AccountView) -> ProgramResult {
    load_config(program_id, cfg)?;
    if !admin.is_signer() { return err(E::NotAdmin); }
    if read_addr(&cfg.try_borrow()?, CFG_ADMIN) != *admin.address() { return err(E::NotAdmin); }
    Ok(())
}

/// 5 AdminSetAllowlist { index u8, kind u8, magic_off u8, magic_len u8, magic [8], program [32] (zero = clear) }
/// accounts: config(w), admin(s)
fn admin_set_allowlist(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    need(accounts, 2)?;
    if data.len() < 44 { return err(E::InvalidInstruction); }
    let (mut cfg, admin) = (accounts[0], accounts[1]);
    check_admin(program_id, &cfg, &admin)?;
    let idx = data[0] as usize;
    if idx >= ALLOW_SLOTS { return err(E::AllowlistIndex); }
    let kind = data[1];
    if !matches!(Kind::from_u8(kind), Some(Kind::Raydium | Kind::Meteora | Kind::Percolator | Kind::Perp)) {
        return err(E::InvalidKind);
    }
    if data[3] > 8 { return err(E::InvalidInstruction); }
    let mut d = cfg.try_borrow_mut()?;
    let base = CFG_ENTRIES + idx * ALLOW_ENTRY_LEN;
    let e = &mut d[base..base + ALLOW_ENTRY_LEN];
    e.fill(0);
    e[..32].copy_from_slice(&data[12..44]);
    e[ENT_KIND] = kind;
    e[ENT_MOFF] = data[2];
    e[ENT_MLEN] = data[3];
    e[ENT_MAGIC..ENT_MAGIC + 8].copy_from_slice(&data[4..12]);
    Ok(())
}

/// 6 AdminUpdate { new_admin [32], paused u8 } — accounts: config(w), admin(s)
fn admin_update(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    need(accounts, 2)?;
    if data.len() < 33 { return err(E::InvalidInstruction); }
    let (mut cfg, admin) = (accounts[0], accounts[1]);
    check_admin(program_id, &cfg, &admin)?;
    let new_admin = read_addr(data, 0);
    let mut d = cfg.try_borrow_mut()?;
    write_addr(&mut d, CFG_ADMIN, &new_admin);
    d[CFG_PAUSED] = data[32];
    Ok(())
}
