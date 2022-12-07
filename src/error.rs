use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntroError {
    #[error("Account not intiialized yet")]
    UninitializedAccount,
    #[error("PDA derived does not equal PDA passed in")]
    InvalidPDA,
    #[error("Input data exceeds max length")]
    InvalidDataLength,
    #[error("Accounts are not same")]
    IncorrectAccountError,
}

impl From<IntroError> for ProgramError {
    fn from(err: IntroError) -> Self {
        ProgramError::Custom(err as u32)
    }
}
