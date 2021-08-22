use crate::error::SomeError;
use crate::instruction::{FomoInstruction, PurchaseKeysParams};
use crate::state::{
    GameState, PlayerRoundState, RoundState, Team, BEAR_FEE_SPLIT, BEAR_POT_SPLIT, GAME_STATE_SIZE,
    INIT_FEE_SPLIT, INIT_POT_SPLIT, PLAYER_ROUND_STATE_SIZE, ROUND_INC_TIME, ROUND_INIT_TIME,
    ROUND_MAX_TIME, ROUND_STATE_SIZE,
};
use crate::util::spl_token::{
    spl_token_init_account, spl_token_transfer, TokenInitializeAccountParams, TokenTransferParams,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
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
use spl_token::state::Account;
use std::ops::Deref;

pub const POT_SEED: &str = "pot";
pub const GAME_STATE_SEED: &str = "game";
pub const ROUND_STATE_SEED: &str = "round";
// pub const PLAYER_STATE_SEED: &str = "player";
pub const PLAYER_ROUND_STATE_SEED: &str = "pr";

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let instruction = FomoInstruction::try_from_slice(data)?;
        match instruction {
            FomoInstruction::InitiateGame(version) => {
                Self::process_initialize_game(program_id, accounts, version)
            }
            FomoInstruction::InitiateRound => Self::process_initialize_round(program_id, accounts),
            FomoInstruction::PurchaseKeys(purchase_params) => {
                Self::process_purchase_keys(program_id, accounts, purchase_params)
            }
            FomoInstruction::WithdrawSol => Self::process_withdraw_sol(program_id, accounts),
        }
    }

    pub fn process_purchase_keys(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        purchase_params: PurchaseKeysParams,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let player_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let player_round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let player_token_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        //todo require player_info to be a signer

        let game_state: GameState = GameState::try_from_slice(&game_state_info.data.borrow_mut())?;
        let player_pk = player_info.key;

        let PurchaseKeysParams { lamports, team } = purchase_params;

        // --------------------------------------- check if player round state exists - if not create
        let player_round_state_seed = format!(
            "{}{}{}{}",
            PLAYER_ROUND_STATE_SEED,      //2
            &player_pk.to_string()[..16], //16 take half the key - should be hard enough to fake
            game_state.round_id,          //8
            game_state.version            //1
        );
        find_and_verify_pda(
            player_round_state_seed.as_bytes(),
            program_id,
            player_round_state_info,
        )?;

        if !check_account_exists(player_round_state_info) {
            create_pda_with_space(
                player_round_state_seed.as_bytes(),
                player_round_state_info,
                PLAYER_ROUND_STATE_SIZE,
                program_id,
                player_info,
                system_program_info,
                program_id,
            )?;
        } else {
            msg!(
                "account for player {} for round {} already exists!",
                player_pk,
                game_state.round_id
            );
        }

        // --------------------------------------- transfer funds to pot
        spl_token_transfer(TokenTransferParams {
            source: player_token_info.clone(),
            destination: pot_info.clone(),
            authority: player_info.clone(),
            token_program: token_program_info.clone(),
            amount: lamports,
            authority_signer_seeds: &[],
        })?;

        // --------------------------------------- prep the variables
        let player_team = match team {
            0 => Team::Whale,
            1 => Team::Bear,
            3 => Team::Bull,
            _ => Team::Snek, //default team snek
        };

        //todo need to calc how many keys they're getting here
        let new_keys = 123;

        // --------------------------------------- serialize round state
        let mut round_state: RoundState =
            RoundState::try_from_slice(&round_state_info.data.borrow_mut())?;
        round_state.lead_player_pk = *player_pk;
        round_state.lead_player_team = player_team;
        //todo needs to be more sophisticated
        round_state.end_time += ROUND_INC_TIME;
        round_state.accum_keys += new_keys;
        round_state.accum_sol_pot += lamports;

        //todo need to calc all the shares

        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        // --------------------------------------- serialize player-round state
        let mut player_round_state: PlayerRoundState =
            PlayerRoundState::try_from_slice(&player_round_state_info.data.borrow_mut())?;

        player_round_state.player_pk = *player_pk;
        player_round_state.round_id = game_state.round_id;
        player_round_state.accum_keys += new_keys;
        player_round_state.accum_sol_added += lamports;

        //todo need to do all the lottery shit

        player_round_state.serialize(&mut *player_round_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_initialize_round(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let funder_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?; //todo can this be replaced with rent sysvar?
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // --------------------------------------- verify game state
        let mut game_state: GameState =
            GameState::try_from_slice(&game_state_info.data.borrow_mut())?;
        let game_state_seed = format!("{}{}", GAME_STATE_SEED, game_state.version);
        find_and_verify_pda(game_state_seed.as_bytes(), program_id, game_state_info)?;

        //todo need some sort of check that the previous round has ended to start a new round
        //todo also some sort of check on who can start the round

        let new_round = game_state.round_id + 1;

        // --------------------------------------- create round state
        let round_state_seed = format!("{}{}{}", ROUND_STATE_SEED, new_round, game_state.version);
        create_pda_with_space(
            round_state_seed.as_bytes(),
            round_state_info,
            ROUND_STATE_SIZE,
            program_id,
            funder_info,
            system_program_info,
            program_id,
        )?;

        // --------------------------------------- create round pot
        let pot_seed = format!("{}{}{}", POT_SEED, new_round, game_state.version);
        create_pda_with_space(
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
            owner: game_state_info.clone(),
            rent: rent_info.clone(),
            token_program: token_program_info.clone(),
        })?;
        //todo transfer money from previous round's pot, if such exists

        // --------------------------------------- update state (NOTE: must go last or get pointer alignment error)

        let pot = Account::unpack(&pot_info.data.borrow_mut())?;
        let clock = Clock::get()?;
        let mut round_state: RoundState =
            RoundState::try_from_slice(&round_state_info.data.borrow_mut())?;

        // all attributes not mentioned automatically start at 0.
        round_state.round_id = game_state.round_id;
        round_state.start_time = clock.unix_timestamp;
        round_state.end_time = round_state.start_time + ROUND_INIT_TIME;
        round_state.ended = false;
        round_state.accum_sol_pot = pot.amount;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        game_state.round_id = new_round;
        game_state.serialize(&mut *game_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_initialize_game(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        version: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let funder_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let game_state_seed = format!("{}{}", GAME_STATE_SEED, version);
        create_pda_with_space(
            game_state_seed.as_bytes(),
            game_state_info,
            GAME_STATE_SIZE,
            program_id,
            funder_info,
            system_program_info,
            program_id,
        )?;

        let mut game_state: GameState =
            GameState::try_from_slice(&game_state_info.data.borrow_mut())?;

        //todo later these can be accepted dynamically
        game_state.round_id = 0; //will be incremented to 1 when 1st round initialized
        game_state.round_init_time = ROUND_INIT_TIME;
        game_state.round_inc_time = ROUND_INC_TIME;
        game_state.round_max_time = ROUND_MAX_TIME;
        game_state.version = version;
        game_state.serialize(&mut *game_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_withdraw_sol(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let game_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let user_info = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        let game_state: GameState = GameState::try_from_slice(&game_state_info.data.borrow_mut())?;

        let game_state_seed = format!("{}{}", GAME_STATE_SEED, game_state.version);
        let game_state_bump =
            find_and_verify_pda(game_state_seed.as_bytes(), program_id, game_state_info)?;
        let pot_seed = format!("{}{}{}", POT_SEED, game_state.round_id, game_state.version);
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
            authority: game_state_info.clone(),
            authority_signer_seeds: &[game_state_seed.as_bytes(), &[game_state_bump]],
            token_program: token_program.clone(),
        };

        spl_token_transfer(spl_transfer_params)?;
        Ok(())
    }

    pub fn process_x(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        Ok(())
    }
}

// ============================================================================= helpers

fn check_account_exists(acc: &AccountInfo) -> bool {
    let does_not_exist = **acc.lamports.borrow() == 0 || acc.data_is_empty();
    !does_not_exist
}

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
