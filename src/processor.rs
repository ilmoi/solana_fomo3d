use crate::error::SomeError;
use crate::instruction::{FomoInstruction, PurchaseKeysParams};
use crate::math::common::{TryCast, TryDiv, TryMul, TrySub};
use crate::math::curve::keys_received;
use crate::state::{
    GameState, PlayerRoundState, RoundState, Team, BEAR_FEE_SPLIT, BEAR_POT_SPLIT, BULL_FEE_SPLIT,
    GAME_STATE_SIZE, PLAYER_ROUND_STATE_SIZE, ROUND_INC_TIME, ROUND_INIT_TIME, ROUND_MAX_TIME,
    ROUND_STATE_SIZE, SNEK_FEE_SPLIT, WHALE_FEE_SPLIT,
};
use crate::util::rng::pseudo_rng;
use crate::util::spl_token::{
    spl_token_init_account, spl_token_transfer, TokenInitializeAccountParams, TokenTransferParams,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::native_token::LAMPORTS_PER_SOL;
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
                msg!("init game");
                Self::process_initialize_game(program_id, accounts, version)
            }
            FomoInstruction::InitiateRound => {
                msg!("init round");
                Self::process_initialize_round(program_id, accounts)
            }
            FomoInstruction::PurchaseKeys(purchase_params) => {
                msg!("purchase keys");
                Self::process_purchase_keys(program_id, accounts, purchase_params)
            }
            FomoInstruction::WithdrawSol => {
                msg!("withdraw sol");
                Self::process_withdraw_sol(program_id, accounts)
            }
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

        let PurchaseKeysParams {
            mut sol_to_be_added,
            team,
            // affiliate_pk,
        } = purchase_params;

        let game_state: GameState = GameState::try_from_slice(&game_state_info.data.borrow_mut())?;
        let player_pk = player_info.key;

        let mut round_state: RoundState =
            RoundState::try_from_slice(&round_state_info.data.borrow_mut())?;

        let mut player_round_state = find_or_create_player_round_state(
            player_round_state_info,
            program_id,
            player_info,
            system_program_info,
            player_pk,
            game_state.round_id,
            game_state.version,
        )?;

        // --------------------------------------- calc & prep vaiables
        // if total pot < 100 sol, each user only allowed to contribute 1 sol total
        sol_to_be_added = if round_state.accum_sol_pot < 100 * LAMPORTS_PER_SOL as u128
            && player_round_state.accum_sol_added + sol_to_be_added > LAMPORTS_PER_SOL as u128
        {
            let allowed_contribution =
                LAMPORTS_PER_SOL as u128 - player_round_state.accum_sol_added;
            allowed_contribution
        } else {
            sol_to_be_added
        };

        let mut fee_split;
        let player_team = match team {
            0 => {
                round_state.accum_sol_by_team.whale += sol_to_be_added;
                fee_split = WHALE_FEE_SPLIT;
                Team::Whale
            }
            1 => {
                round_state.accum_sol_by_team.bear += sol_to_be_added;
                fee_split = BEAR_FEE_SPLIT;
                Team::Bear
            }
            3 => {
                round_state.accum_sol_by_team.bull += sol_to_be_added;
                fee_split = BULL_FEE_SPLIT;
                Team::Bull
            }
            _ => {
                round_state.accum_sol_by_team.snek += sol_to_be_added;
                fee_split = SNEK_FEE_SPLIT;
                Team::Snek
            }
        };

        // Ensure enough lamports are sent to buy at least 1 whole key.
        // In the original game on Ethereum it was possible to purchase <1 key.
        // On Solana however, due to restrictions around doing U256 math, we set the min as 1 key.
        // In practice this means a min participation ticket of:
        //  - 75_000 lamports/key at the beginning of the round (when keys are cheap)
        //  - 1.7 sol/per at max capacity of the game (10bn SOL total - not actually achievable)
        let new_keys = keys_received(round_state.accum_sol_pot, sol_to_be_added)?;
        if new_keys < 1 {
            msg!("your purchase is too small - min 1 key");
            return Err(SomeError::BadError.into());
        }

        // --------------------------------------- transfer funds to pot
        spl_token_transfer(TokenTransferParams {
            source: player_token_info.clone(),
            destination: pot_info.clone(),
            authority: player_info.clone(), //this automatically enforces player_info is a signer, thus verifying that acc
            token_program: token_program_info.clone(),
            amount: sol_to_be_added.try_cast()?,
            authority_signer_seeds: &[],
        })?;

        // --------------------------------------- part take in airdrop lottery
        //if they deposited > 0.1 sol, they're eligible for airdrop
        if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_div(10)? {
            let clock = Clock::get()?;

            //with every extra player chance of airdrop increases by 0.1%
            round_state.airdrop_tracker += 1;

            if airdrop_winner(player_pk, &clock, round_state.airdrop_tracker)? {
                let airdrop_to_distribute = round_state.accum_airdrop_share;
                //3 tiers exist for airdrop
                let prize = if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_mul(10)? {
                    //10+ sol - win 75% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(75)?.try_div(100)?
                } else if sol_to_be_added > LAMPORTS_PER_SOL as u128 {
                    //1-10 sol - win 50% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(50)?.try_div(100)?
                } else {
                    //0.1-1 sol - win 25% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(25)?.try_div(100)?
                };

                //send money
                round_state.accum_airdrop_share -= prize;
                player_round_state.accum_winnings += prize;
                //restart the lottery
                round_state.airdrop_tracker = 0;
            }
        }

        // --------------------------------------- split the fee among stakeholders
        //2% to community
        //todo impl mechanism where only community member can withdraw
        let community_share = sol_to_be_added.try_div(50)?;
        //1% to future airdrops
        let airdrop_share = sol_to_be_added.try_div(100)?;
        //1% to next round's pot
        let next_round_share = sol_to_be_added.try_div(100)?;
        //10% to affiliate
        let affiliate_share = sol_to_be_added.try_div(10)?;

        //todo impl mechanism where only p3d member can withdraw
        let mut p3d_share = 0;
        let mut f3d_share = 0;

        //if player has an affiliate listed, record their share, else share goes to p3d holders
        if player_round_state.has_affiliate() {
            //optional account passed only if affiliate listed
            let affiliate_round_state_info = next_account_info(account_info_iter)?;
            let mut affiliate_round_state = find_or_create_player_round_state(
                affiliate_round_state_info,
                program_id,
                player_info,
                system_program_info,
                &player_round_state.last_affiliate_pk,
                game_state.round_id,
                game_state.version,
            )?;
            affiliate_round_state.accum_aff += affiliate_share;
            round_state.accum_aff_share += affiliate_share;
        } else {
            p3d_share += affiliate_share;
        }

        p3d_share += sol_to_be_added
            .try_mul(fee_split.p3d as u128)?
            .try_div(100)?;
        f3d_share += sol_to_be_added
            .try_mul(fee_split.f3d as u128)?
            .try_div(100)?;
        let prize_share = sol_to_be_added
            .try_sub(community_share)?
            .try_sub(airdrop_share)?
            .try_sub(next_round_share)?
            .try_sub(affiliate_share)?
            .try_sub(p3d_share)?
            .try_sub(f3d_share)?;

        // --------------------------------------- serialize round state
        //update leader
        round_state.lead_player_pk = *player_pk;
        round_state.lead_player_team = player_team;
        //update timer - todo needs to be more sophisticated
        round_state.end_time += ROUND_INC_TIME;
        //update totals
        round_state.accum_keys += new_keys;
        round_state.accum_sol_pot += sol_to_be_added;
        //distribute shares
        round_state.accum_community_share += community_share;
        round_state.accum_airdrop_share += airdrop_share;
        round_state.accum_next_round_share += next_round_share;
        round_state.accum_aff_share += affiliate_share;
        round_state.accum_p3d_share += p3d_share;
        round_state.accum_f3d_share += f3d_share;
        round_state.accum_prize_share += prize_share;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        verify_round_state(&round_state, pot_info);

        // --------------------------------------- serialize player-round state
        //update totals
        player_round_state.accum_keys += new_keys;
        player_round_state.accum_sol_added += sol_to_be_added;
        player_round_state.serialize(&mut *player_round_state_info.data.borrow_mut())?;

        //todo impl a check for the math

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
        let rent_info = next_account_info(account_info_iter)?;
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
        round_state.accum_sol_pot = pot.amount as u128;
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

        //todo need a check to ensure the pk listed on this account is passed in as a signer - else anyone could withdraw

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

/// Checks whether actual funds in the pot equate to total of all the parties' shares.
fn verify_round_state(round_state: &RoundState, pot_info: &AccountInfo) -> ProgramResult {
    let actual_money_in_pot = Account::unpack(&pot_info.data.borrow())?.amount;
    let supposed_money_in_pot = round_state.accum_community_share
        + round_state.accum_airdrop_share
        + round_state.accum_next_round_share
        + round_state.accum_aff_share
        + round_state.accum_p3d_share
        + round_state.accum_f3d_share
        + round_state.accum_prize_share;
    msg!("{}, {}", actual_money_in_pot as u128, supposed_money_in_pot);
    assert_eq!(actual_money_in_pot as u128, supposed_money_in_pot);
    Ok(())
}

fn find_or_create_player_round_state<'a>(
    player_round_state_info: &AccountInfo<'a>,
    program_id: &Pubkey,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    player_pk: &Pubkey,
    round_id: u64,
    version: u8,
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

fn airdrop_winner(
    player_pk: &Pubkey,
    clock: &Clock,
    airdrop_tracker: u64,
) -> Result<bool, ProgramError> {
    let lottery_ticket = pseudo_rng(player_pk, clock)?;
    Ok(lottery_ticket < airdrop_tracker as u128)
}

fn account_exists(acc: &AccountInfo) -> bool {
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
