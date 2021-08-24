use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction::create_account,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::state::Account;

use crate::{
    error::SomeError,
    processor::{
        spl_token::{spl_token_init_account, TokenInitializeAccountParams},
        util::account_exists,
    },
    state::{
        GameState, PlayerRoundState, RoundState, GAME_STATE_SIZE, PLAYER_ROUND_STATE_SIZE,
        ROUND_STATE_SIZE,
    },
};

// --------------------------------------- public

/// Builds seed + verifies + deserializes pda
pub fn deserialize_game_state<'a>(
    game_state_info: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> Result<(GameState, String, u8), ProgramError> {
    let game_state: GameState = GameState::try_from_slice(&game_state_info.data.borrow_mut())?;
    let game_state_seed = format!("{}{}", GAME_STATE_SEED, game_state.version);
    let game_state_bump =
        find_and_verify_pda(game_state_seed.as_bytes(), program_id, game_state_info)?;
    Ok((game_state, game_state_seed, game_state_bump))
}

/// Builds seed + verifies + creates pda
pub fn create_game_state<'a>(
    game_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    version: u8,
    program_id: &Pubkey,
) -> Result<GameState, ProgramError> {
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
    GameState::try_from_slice(&game_state_info.data.borrow_mut())
        .map_err(|_| SomeError::BadError.into())
}

/// Builds seed + verifies + deserializes pda
pub fn deserialize_round_state<'a>(
    round_state_info: &AccountInfo<'a>,
    round_id: u64,
    game_version: u8,
    program_id: &Pubkey,
) -> Result<RoundState, ProgramError> {
    let round_state: RoundState = RoundState::try_from_slice(&round_state_info.data.borrow_mut())?;
    let round_state_seed = format!("{}{}{}", ROUND_STATE_SEED, round_id, game_version);
    find_and_verify_pda(round_state_seed.as_bytes(), program_id, round_state_info)?;
    Ok(round_state)
}

/// Builds seed + verifies + creates pda
pub fn create_round_state<'a>(
    round_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    round_id: u64,
    version: u8,
    program_id: &Pubkey,
) -> Result<RoundState, ProgramError> {
    let round_state_seed = format!("{}{}{}", ROUND_STATE_SEED, round_id, version);
    create_pda_with_space(
        round_state_seed.as_bytes(),
        round_state_info,
        ROUND_STATE_SIZE,
        program_id,
        funder_info,
        system_program_info,
        program_id,
    )?;
    RoundState::try_from_slice(&round_state_info.data.borrow_mut())
        .map_err(|_| SomeError::BadError.into())
}

/// Builds seed + verifies + deserializes pda
pub fn deserialize_pot<'a>(
    pot_info: &AccountInfo<'a>,
    round_id: u64,
    game_version: u8,
    program_id: &Pubkey,
) -> Result<Account, ProgramError> {
    // todo this check will fail coz owner = token_program. I wonder if there is another check that I need to do in place
    // if *pot_info.owner != *fomo3d_state_info.key {
    //     msg!("owner of pot account is not fomo3d");
    //     return Err(SomeError::BadError.into());
    // }

    let pot = Account::unpack(&pot_info.data.borrow_mut())?;
    let pot_seed = format!("{}{}{}", POT_SEED, round_id, game_version);
    find_and_verify_pda(pot_seed.as_bytes(), program_id, pot_info)?;
    Ok(pot)
}

/// Builds seed + verifies + creates pda
pub fn create_pot<'a>(
    pot_info: &AccountInfo<'a>,
    game_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    mint_info: &AccountInfo<'a>,
    rent_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    round_id: u64,
    version: u8,
    program_id: &Pubkey,
) -> Result<Account, ProgramError> {
    let pot_seed = format!("{}{}{}", POT_SEED, round_id, version);
    create_pda_with_space(
        pot_seed.as_bytes(),
        pot_info,
        spl_token::state::Account::get_packed_len(),
        &spl_token::id(),
        funder_info,
        system_program_info,
        program_id,
    )?;
    // initialize + give game_state pda "ownership" over it
    spl_token_init_account(TokenInitializeAccountParams {
        account: pot_info.clone(),
        mint: mint_info.clone(),
        owner: game_state_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;
    Account::unpack(&pot_info.data.borrow_mut()).map_err(|_| SomeError::BadError.into())
}

/// Builds seed + verifies + deserializes/creates pda if missing
pub fn deserialize_or_create_player_round_state<'a>(
    player_round_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    player_pk: &Pubkey,
    round_id: u64,
    version: u8,
    program_id: &Pubkey,
) -> Result<PlayerRoundState, ProgramError> {
    let player_round_state_seed = format!(
        "{}{}{}{}",
        PLAYER_ROUND_STATE_SEED,      //2
        &player_pk.to_string()[..16], //16 take half the key - should be hard enough to fake
        round_id,                     //8
        version                       //1
    );

    find_and_verify_pda(
        player_round_state_seed.as_bytes(),
        program_id,
        player_round_state_info,
    )?;

    if !account_exists(player_round_state_info) {
        create_pda_with_space(
            player_round_state_seed.as_bytes(),
            player_round_state_info,
            PLAYER_ROUND_STATE_SIZE,
            program_id,
            funder_info,
            system_program_info,
            program_id,
        )?;
        let mut player_round_state: PlayerRoundState =
            PlayerRoundState::try_from_slice(&player_round_state_info.data.borrow_mut())?;
        //initially set the player's public key and round id
        player_round_state.player_pk = *player_pk;
        player_round_state.round_id = round_id;
        Ok(player_round_state)
    } else {
        msg!(
            "account for player {} for round {} already exists!",
            player_pk,
            round_id
        );
        PlayerRoundState::try_from_slice(&player_round_state_info.data.borrow_mut())
            .map_err(|_| SomeError::BadError.into())
    }
}

// --------------------------------------- private

const POT_SEED: &str = "pot";
const GAME_STATE_SEED: &str = "game";
const ROUND_STATE_SEED: &str = "round";
const PLAYER_ROUND_STATE_SEED: &str = "pr";

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
