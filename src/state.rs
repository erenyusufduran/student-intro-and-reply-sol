use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StudentIntroState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub writer: Pubkey,
    pub name: String,
    pub message: String,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StudentReplyState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub intro: Pubkey,
    pub replier: Pubkey,
    pub name: String,
    pub message: String,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ReplyCount {
    pub discriminator: String,
    pub is_initialized: bool,
    pub counter: u64,
}

impl Sealed for StudentIntroState {}

impl Sealed for ReplyCount {}

impl IsInitialized for StudentIntroState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for StudentReplyState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for ReplyCount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl StudentIntroState {
    pub const DISCRIMINATOR: &'static str = "intro";

    pub fn get_account_size(name: String, message: String) -> usize {
        return (4 + StudentIntroState::DISCRIMINATOR.len())
            + 1
            + 32
            + (4 + name.len())
            + (4 + message.len());
    }
}

impl StudentReplyState {
    pub const DISCRIMINATOR: &'static str = "reply";

    pub fn get_account_size(name: String, message: String) -> usize {
        return (4 + StudentReplyState::DISCRIMINATOR.len())
            + 1
            + 32
            + 32
            + (4 + name.len())
            + (4 + message.len());
    }
}

impl ReplyCount {
    pub const DISCRIMINATOR: &'static str = "counter";
    pub const SIZE: usize = (4 + ReplyCount::DISCRIMINATOR.len()) + 1 + 8;
}
