use pinocchio::Address;

pub const CONFIG_SEED: &[u8] = b"config";
pub const PAIRING_SEED: &[u8] = b"pairing";
pub const PRIMARY_SEED: &[u8] = b"primary";
pub const ASSET_SIGNER_SEED: &[u8] = b"mpl-core-execute";

pub const KEY_CONFIG: u8 = 1;
pub const KEY_PAIRING: u8 = 2;
pub const KEY_PRIMARY: u8 = 3;

pub const VERSION: u8 = 1;

// ---------------------------------------------------------------- config
pub const ALLOW_SLOTS: usize = 16;
pub const ALLOW_ENTRY_LEN: usize = 48;
pub const CONFIG_LEN: usize = 40 + ALLOW_SLOTS * ALLOW_ENTRY_LEN; // 808
// layout: key u8 | bump u8 | version u8 | paused u8 | pad[4] | admin [32] | entries
pub const CFG_PAUSED: usize = 3;
pub const CFG_ADMIN: usize = 8;
pub const CFG_ENTRIES: usize = 40;
// entry: program [32] | kind u8 | magic_off u8 | magic_len u8 | pad[5] | magic [8]
pub const ENT_KIND: usize = 32;
pub const ENT_MOFF: usize = 33;
pub const ENT_MLEN: usize = 34;
pub const ENT_MAGIC: usize = 40;

// ---------------------------------------------------------------- pairing
pub const PAIRING_LEN: usize = 208;
pub const P_KEY: usize = 0;
pub const P_BUMP: usize = 1;
pub const P_KIND: usize = 2;
pub const P_FLAGS: usize = 3;
pub const P_ASSET: usize = 8;
pub const P_TARGET: usize = 40;
pub const P_TARGET_PROGRAM: usize = 72;
pub const P_RESOLVED_MINT: usize = 104;
pub const P_SET_BY: usize = 136;
pub const P_SLOT: usize = 168;
// 176..208 reserved

pub const FLAG_LOCKED: u8 = 1 << 0;
pub const FLAG_PRIMARY: u8 = 1 << 1;
pub const FLAG_VERIFIED_MINT: u8 = 1 << 2;
pub const FLAG_UNVERIFIED_TARGET: u8 = 1 << 3;
pub const FLAG_VIA_ASSET_SIGNER: u8 = 1 << 4;

// ---------------------------------------------------------------- primary pointer
pub const PRIMARY_LEN: usize = 48;
pub const PP_PAIRING: usize = 8;
pub const PP_SLOT: usize = 40;

// ---------------------------------------------------------------- kinds
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Spl = 0,
    Token2022 = 1,
    PumpCurve = 2,
    PumpSwap = 3,
    Raydium = 4,
    Meteora = 5,
    Genesis = 6,
    Percolator = 7,
    Perp = 8,
    Other = 9,
}

impl Kind {
    pub fn from_u8(v: u8) -> Option<Kind> {
        Some(match v {
            0 => Kind::Spl,
            1 => Kind::Token2022,
            2 => Kind::PumpCurve,
            3 => Kind::PumpSwap,
            4 => Kind::Raydium,
            5 => Kind::Meteora,
            6 => Kind::Genesis,
            7 => Kind::Percolator,
            8 => Kind::Perp,
            9 => Kind::Other,
            _ => return None,
        })
    }
}

// ---------------------------------------------------------------- well-known programs (mainnet IDs; most are also on devnet)
pub const SYSTEM_PROGRAM: Address = Address::new_from_array([0u8; 32]);
pub const CORE: Address = Address::from_str_const("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
pub const BPF_UPGRADEABLE_LOADER: Address = Address::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
pub const TOKEN: Address = Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022: Address = Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const PUMP: Address = Address::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMP_SWAP: Address = Address::from_str_const("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const GENESIS: Address = Address::from_str_const("GNS1S5J5AspKXgpjz6SvKL66kPaKWAhaGRhCqPRxii2B");
pub const PHOENIX: Address = Address::from_str_const("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
pub const RAYDIUM: [Address; 4] = [
    Address::from_str_const("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"), // AMM v4
    Address::from_str_const("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"), // CPMM
    Address::from_str_const("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"), // CLMM
    Address::from_str_const("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj"),  // LaunchLab
];
pub const METEORA: [Address; 4] = [
    Address::from_str_const("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"),  // DLMM
    Address::from_str_const("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB"), // DAMM v1
    Address::from_str_const("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"),  // DAMM v2
    Address::from_str_const("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"),  // DBC
];

// ---------------------------------------------------------------- byte helpers
#[inline(always)]
pub fn read_addr(d: &[u8], off: usize) -> Address {
    let mut a = [0u8; 32];
    a.copy_from_slice(&d[off..off + 32]);
    Address::new_from_array(a)
}

#[inline(always)]
pub fn write_addr(d: &mut [u8], off: usize, a: &Address) {
    d[off..off + 32].copy_from_slice(a.as_ref());
}
