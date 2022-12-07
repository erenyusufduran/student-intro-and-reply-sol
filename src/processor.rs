use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

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
    let system_program = next_account_info(account_info_iter)?;

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
        Pubkey::find_program_address(&[pda.as_ref(), "comment".as_ref()], program_id);

    if pda_count != *counter_pda.key {
        msg!("Invalid seeds for counter PDA.");
        return Err(ProgramError::InvalidArgument);
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
    let system_program = next_account_info(account_info_iter)?;

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

    Ok(())
}
