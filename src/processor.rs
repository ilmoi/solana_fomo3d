use crate::error::SomeError;
use crate::instruction::FomoInstruction;
use crate::state::Fomo3dState;
use crate::util::spl_token::{
    spl_token_init_account, spl_token_transfer, TokenInitializeAccountParams, TokenTransferParams,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::{create_account, transfer, transfer_with_seed};
use solana_program::sysvar::rent::Rent;
use solana_program::sysvar::Sysvar;

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let instruction = FomoInstruction::try_from_slice(data)?;
        match instruction {
            FomoInstruction::Initialize(x) => Self::process_initialize(program_id, accounts, x),
            FomoInstruction::PurchaseKeys(sol_amount) => {
                Self::process_purchase_keys(program_id, accounts, sol_amount)
            }
            FomoInstruction::PayOut(x) => Self::process_pay_out(program_id, accounts, x),
        }
    }

    pub fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        x: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let funder_info = next_account_info(account_info_iter)?;
        let state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?; //todo can this be replaced with rent sysvar?
        let token_program_info = next_account_info(account_info_iter)?;

        // --------------------------------------- prepare state account
        let state_seed = format!("state{}", x); //todo temp
        let state_bump = create_pda_with_space(
            state_seed.as_bytes(),
            state_info,
            34, //todo can this be done dynamically via borsh?
            program_id,
            funder_info,
            system_program_info,
            program_id,
        )?;

        // --------------------------------------- prepare pot account
        let pot_seed = format!("pot{}", x); //todo temp
        let pot_bump = create_pda_with_space(
            pot_seed.as_bytes(),
            pot_info,
            spl_token::state::Account::get_packed_len(),
            &spl_token::id(),
            funder_info,
            system_program_info,
            program_id,
        )?;
        // initialize + give the pda "ownership" over it
        spl_token_init_account(TokenInitializeAccountParams {
            account: pot_info.clone(),
            mint: mint_info.clone(),
            owner: state_info.clone(),
            rent: rent_info.clone(),
            token_program: token_program_info.clone(),
        })?;

        // --------------------------------------- write state

        let mut fomo3d_state: Fomo3dState =
            Fomo3dState::try_from_slice(&state_info.data.borrow_mut())?;

        //todo later these can be accepted dynamically
        fomo3d_state.round_id = 1;
        fomo3d_state.round_init_time = 1 * 60 * 60;
        fomo3d_state.round_inc_time = 30;
        fomo3d_state.round_max_time = 24 * 60 * 60;
        fomo3d_state.state_bump = state_bump;
        fomo3d_state.pot_bump = pot_bump;

        fomo3d_state.serialize(&mut *state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_pay_out(program_id: &Pubkey, accounts: &[AccountInfo], x: u8) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let user_info = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        let state_seed = format!("state{}", x); //todo temp
        let pot_seed = format!("pot{}", x); //todo temp
        let state_bump = find_and_verify_pda(state_seed.as_bytes(), program_id, state_info)?;
        find_and_verify_pda(pot_seed.as_bytes(), program_id, pot_info)?;

        // todo this check will fail coz owner = token_program. I wonder if there is another check that I need to do in place
        // if *pot_info.owner != *fomo3d_state_info.key {
        //     msg!("owner of pot account is not fomo3d");
        //     return Err(SomeError::BadError.into());
        // }

        let spl_transfer_params = TokenTransferParams {
            source: pot_info.clone(),
            destination: user_info.clone(),
            amount: 1,
            authority: state_info.clone(),
            authority_signer_seeds: &[state_seed.as_bytes(), &[state_bump]],
            token_program: token_program.clone(),
        };

        spl_token_transfer(spl_transfer_params)?;
        Ok(())
    }

    pub fn process_purchase_keys(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        sol_amount: u64,
    ) -> ProgramResult {
        Ok(())
    }

    pub fn process_x(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        Ok(())
    }
}

// ============================================================================= helpers

fn find_and_verify_pda(
    pda_seed: &[u8],
    program_id: &Pubkey,
    pda_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (pda, bump_seed) = Pubkey::find_program_address(&[pda_seed], program_id);
    if pda != *pda_info.key {
        msg!("pda doesnt match: {}, {}", pda, *pda_info.key);
        return Err(SomeError::BadError.into());
    }
    Ok(bump_seed)
}

fn create_pda_with_space<'a>(
    pda_seed: &[u8],
    pda_info: &AccountInfo<'a>,
    space: usize,
    owner: &Pubkey,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let bump_seed = find_and_verify_pda(pda_seed, program_id, pda_info)?;
    let full_seeds: &[&[_]] = &[pda_seed, &[bump_seed]];

    //create a PDA and allocate space inside of it at the same time
    //can only be done from INSIDE the program
    invoke_signed(
        &create_account(
            &funder_info.key,
            &pda_info.key,
            1.max(Rent::get()?.minimum_balance(space)),
            space as u64,
            owner,
        ),
        &[
            //yes need all three
            //https://github.com/solana-labs/solana-program-library/blob/7c8e65292a6ebc90de54468c665e30bc590c513a/feature-proposal/program/src/processor.rs#L148-L163
            //(!) need to do .clone() even though we did .clone() to pass in the args - otherwise get an error around access violation
            funder_info.clone(),
            pda_info.clone(),
            system_program_info.clone(),
        ],
        &[full_seeds], //this is the part you can't do outside the program
    )?;

    msg!("pda created");
    Ok(bump_seed)
}

//todo add rent checks
//todo add owner checks + other checks from eg token-lending
//todo https://blog.neodyme.io/posts/solana_common_pitfalls#solana-account-confusions
