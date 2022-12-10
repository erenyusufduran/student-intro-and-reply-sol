use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    system_program::ID as SYSTEM_PROGRAM_ID,
    sysvar::{rent::Rent, rent::ID as RENT_PROGRAM_ID, Sysvar},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{instruction::initialize_mint, ID as TOKEN_PROGRAM_ID};

use crate::{
    error::IntroError,
    instruction::StudentInstruction,
    state::{ReplyCount, StudentIntroState, StudentReplyState},
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = StudentInstruction::unpack(instruction_data)?;

    match instruction {
        StudentInstruction::StudentIntro { name, message } => {
            student_intro(program_id, accounts, name, message)
        }

        StudentInstruction::UpdateIntro { name, message } => {
            update_intro(program_id, accounts, name, message)
        }

        StudentInstruction::ReplyIntro { name, message } => {
            reply_intro(program_id, accounts, name, message)
        }

        StudentInstruction::InitializeMint => initialize_token_mint(program_id, accounts),
    }
}

pub fn student_intro(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    message: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let writer = next_account_info(account_info_iter)?;
    let intro_pda = next_account_info(account_info_iter)?;
    let counter_pda = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    if !writer.is_signer {
        msg!("Missing required signature");
        return Err(solana_program::program_error::ProgramError::MissingRequiredSignature);
    }

    let (pda, bump_seed) =
        Pubkey::find_program_address(&[writer.key.as_ref(), "intro".as_ref()], program_id);

    if pda != *intro_pda.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let (pda_count, _counter_bump_seed) =
        Pubkey::find_program_address(&[pda.as_ref(), "counter".as_ref()], program_id);

    if pda_count != *counter_pda.key {
        msg!("Invalid seeds for counter PDA.");
        return Err(ProgramError::InvalidArgument);
    }

    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    if mint_pda != *token_mint.key {
        msg!("Incorrect token mint");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if mint_auth_pda != *mint_auth.key {
        msg!("Mint passed in and mint derived do not match");
        return Err(IntroError::InvalidPDA.into());
    }

    if *user_ata.key != get_associated_token_address(writer.key, token_mint.key) {
        msg!("Incorrect token mint");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(IntroError::IncorrectAccountError.into());
    }

    let account_len: usize = 1000;

    if (StudentIntroState::get_account_size(name.clone(), message.clone())) > account_len {
        msg!("Data length is larger than 1000 bytes");
        return Err(IntroError::InvalidDataLength.into());
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);
    let counter_rent_lamp = rent.minimum_balance(ReplyCount::SIZE);

    invoke_signed(
        &system_instruction::create_account(
            writer.key,
            intro_pda.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[writer.clone(), intro_pda.clone(), system_program.clone()],
        &[&[
            writer.key.as_ref(),
            "intro".as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;
    msg!("Intro PDA Created: {}", pda);

    invoke_signed(
        &system_instruction::create_account(
            writer.key,
            counter_pda.key,
            counter_rent_lamp,
            ReplyCount::SIZE.try_into().unwrap(),
            program_id,
        ),
        &[writer.clone(), counter_pda.clone(), system_program.clone()],
        &[&[pda.as_ref(), "counter".as_ref(), &[_counter_bump_seed]]],
    )?;
    msg!("Reply Counter Created: {}", pda_count);

    let mut intro_data =
        try_from_slice_unchecked::<StudentIntroState>(&intro_pda.data.borrow()).unwrap();
    let mut counter_data =
        try_from_slice_unchecked::<ReplyCount>(&counter_pda.data.borrow()).unwrap();

    if intro_data.is_initialized() {
        msg!("Account already initialized!");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if counter_data.is_initialized() {
        msg!("Counter already initialized!");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    intro_data.discriminator = StudentIntroState::DISCRIMINATOR.to_string();
    intro_data.writer = *writer.key;
    intro_data.name = name;
    intro_data.message = message;
    intro_data.is_initialized = true;

    counter_data.discriminator = ReplyCount::DISCRIMINATOR.to_string();
    counter_data.counter = 0;
    counter_data.is_initialized = true;

    msg!("Reply Count: {}", counter_data.counter);

    intro_data.serialize(&mut &mut intro_pda.data.borrow_mut()[..])?;
    counter_data.serialize(&mut &mut counter_pda.data.borrow_mut()[..])?;

    msg!("Minting 10 tokens to user associated token account.");
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            10 * LAMPORTS_PER_SOL,
        )?,
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;

    Ok(())
}

pub fn update_intro(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    message: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let writer = next_account_info(account_info_iter)?;
    let pda_intro = next_account_info(account_info_iter)?;

    if pda_intro.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    if !writer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut intro_data =
        try_from_slice_unchecked::<StudentIntroState>(&pda_intro.data.borrow()).unwrap();

    let (pda, _bump_seed) =
        Pubkey::find_program_address(&[writer.key.as_ref(), "intro".as_ref()], program_id);

    if pda != *pda_intro.key {
        msg!("Invalid seeds for PDA");
        return Err(IntroError::InvalidPDA.into());
    }

    if !intro_data.is_initialized() {
        msg!("Account is not initialized");
        return Err(IntroError::UninitializedAccount.into());
    }

    if StudentIntroState::get_account_size(name.clone(), message.clone()) > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(IntroError::InvalidDataLength.into());
    }

    intro_data.message = message;
    intro_data.serialize(&mut &mut pda_intro.data.borrow_mut()[..])?;

    Ok(())
}

pub fn reply_intro(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    message: String,
) -> ProgramResult {
    msg!("{}, {}, {}", program_id, name, message);
    let account_info_iter = &mut accounts.iter();

    let replier = next_account_info(account_info_iter)?;
    let pda_intro = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let pda_reply = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    if *token_mint.key != mint_pda {
        msg!("Incorrect token mint");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *mint_auth.key != mint_auth_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *user_ata.key != get_associated_token_address(replier.key, token_mint.key) {
        msg!("Incorrect token mint");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(IntroError::IncorrectAccountError.into());
    }

    let mut counter_data =
        try_from_slice_unchecked::<ReplyCount>(&pda_counter.data.borrow()).unwrap();

    let account_len = StudentReplyState::get_account_size(name.clone(), message.clone());
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[
            pda_intro.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if pda != *pda_reply.key {
        msg!("Invalid seeds for PDA");
        return Err(IntroError::InvalidPDA.into());
    }

    invoke_signed(
        &system_instruction::create_account(
            replier.key,
            pda_reply.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[replier.clone(), pda_reply.clone(), system_program.clone()],
        &[&[
            pda_intro.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;
    msg!("Created Reply Account");

    let mut reply_data =
        try_from_slice_unchecked::<StudentReplyState>(&pda_reply.data.borrow()).unwrap();

    if reply_data.is_initialized() {
        msg!("Account already intitialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    reply_data.discriminator = StudentReplyState::DISCRIMINATOR.to_string();
    reply_data.intro = *pda_intro.key;
    reply_data.replier = *replier.key;
    reply_data.name = name;
    reply_data.message = message;
    reply_data.is_initialized = true;

    counter_data.counter += 1;

    reply_data.serialize(&mut &mut pda_reply.data.borrow_mut()[..])?;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    msg!("Minting 5 tokens to user associated token account");
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            5 * LAMPORTS_PER_SOL,
        )?,
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;

    Ok(())
}

pub fn initialize_token_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let sysvar_rent = next_account_info(account_info_iter)?;

    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, _mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    msg!("Token mint: {:?}", mint_pda);
    msg!("Mint authority: {:?}", mint_auth_pda);

    if mint_pda != *token_mint.key {
        msg!("Incorrect token mint account");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *mint_auth.key != mint_auth_pda {
        msg!("Incorrect mint auth account");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *system_program.key != SYSTEM_PROGRAM_ID {
        msg!("Incorrect system program");
        return Err(IntroError::IncorrectAccountError.into());
    }

    if *sysvar_rent.key != RENT_PROGRAM_ID {
        msg!("Incorrect rent program");
        return Err(IntroError::IncorrectAccountError.into());
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(82);

    // create the token mint PDA.
    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            token_mint.key,
            rent_lamports,
            82, // Size of the token mint account
            token_program.key,
        ),
        // Accounts we're reading from or writing to
        &[
            initializer.clone(),
            token_mint.clone(),
            system_program.clone(),
        ],
        // Seeds for our token mint account
        &[&[b"token_mint", &[mint_bump]]],
    )?;
    msg!("Created token mint account");

    // Initialize the mint account
    invoke_signed(
        &initialize_mint(
            token_program.key,
            token_mint.key,
            mint_auth.key,
            Option::None, // Freeze authority - we don't want anyone to be able to freeze!
            9,            // Number of decimals
        )?,
        // Which accounts we're reading from or writing to
        &[token_mint.clone(), sysvar_rent.clone(), mint_auth.clone()],
        // The seeds for our token mint PDA
        &[&[b"token_mint", &[mint_bump]]],
    )?;
    msg!("Initialized token mint");

    Ok(())
}
