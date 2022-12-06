use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

pub enum StudentInstruction {
    StudentIntro { name: String, message: String },
    UpdateIntro { name: String, message: String },
    ReplyIntro { name: String, message: String },
}

#[derive(BorshDeserialize)]
struct StudentIntroPayload {
    name: String,
    message: String,
}

impl StudentInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        let payload = StudentIntroPayload::try_from_slice(rest).unwrap();

        Ok(match variant {
            0 => Self::StudentIntro {
                name: payload.name,
                message: payload.message,
            },
            1 => Self::UpdateIntro {
                name: payload.name,
                message: payload.message,
            },
            2 => Self::ReplyIntro {
                name: payload.name,
                message: payload.message,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
