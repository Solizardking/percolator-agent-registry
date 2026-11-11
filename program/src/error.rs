use pinocchio::error::ProgramError;

/// Custom error codes (returned as `Custom(n)`).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    InvalidInstruction = 0,
    NotEnoughAccounts = 1,
    InvalidAsset = 2,
    Unauthorized = 3,
    InvalidPda = 4,
    InvalidKind = 5,
    TargetOwnerMismatch = 6,
    TargetLayoutInvalid = 7,
    ResolvedMintMismatch = 8,
    ResolvedMintRequired = 9,
    PairingLocked = 10,
    PrimaryLocked = 11,
    AlreadyInitialized = 12,
    NotInitialized = 13,
    Paused = 14,
    NotAdmin = 15,
    InvalidProgramData = 16,
    NotUpgradeAuthority = 17,
    AllowlistIndex = 18,
    WrongPairing = 19,
    TargetMissing = 20,
}

impl From<RegistryError> for ProgramError {
    fn from(e: RegistryError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[inline(always)]
pub fn err<T>(e: RegistryError) -> Result<T, ProgramError> {
    Err(e.into())
}
