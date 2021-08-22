use solana_program::pubkey::Pubkey;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::entrypoint;
use crate::processor;
use solana_program::program_error::PrintProgramError;
use crate::error::SomeError;

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if let Err(e) = processor::Processor::process_instruction(program_id, accounts, data) {
        e.print::<SomeError>();
        return Err(e);
    }
    Ok(())
}