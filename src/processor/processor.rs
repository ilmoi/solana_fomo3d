use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::SomeError,
    instruction::{GameInstruction, PurchaseKeysParams, WithdrawSolParams},
    math::{
        common::{TryAdd, TryCast, TryDiv, TryMul, TrySub},
        curve::keys_received,
    },
    processor::{
        pda::{
            create_game_state, create_pot, create_round_state, deserialize_game_state,
            deserialize_or_create_player_round_state, deserialize_pot, deserialize_round_state,
        },
        spl_token::{spl_token_transfer, TokenTransferParams},
        util::{airdrop_winner, calculate_player_f3d_share, verify_round_state},
    },
    state::{
        Team, BEAR_FEE_SPLIT, BEAR_POT_SPLIT, BULL_FEE_SPLIT, ROUND_INC_TIME, ROUND_INIT_TIME,
        ROUND_MAX_TIME, SNEK_FEE_SPLIT, WHALE_FEE_SPLIT,
    },
};

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let instruction = GameInstruction::try_from_slice(data)?;
        match instruction {
            GameInstruction::InitiateGame(version) => {
                msg!("init game");
                Self::process_initialize_game(program_id, accounts, version)
            }
            GameInstruction::InitiateRound => {
                msg!("init round");
                Self::process_initialize_round(program_id, accounts)
            }
            GameInstruction::PurchaseKeys(purchase_params) => {
                msg!("purchase keys");
                Self::process_purchase_keys(program_id, accounts, purchase_params)
            }
            GameInstruction::WithdrawSol(withdraw_params) => {
                msg!("withdraw sol");
                Self::process_withdraw_sol(program_id, accounts, withdraw_params)
            }
            GameInstruction::EndRound => {
                msg!("end round");
                Self::process_end_round(program_id, accounts)
            }
        }
    }

    pub fn process_end_round(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let _game_state_info = next_account_info(account_info_iter)?;
        let _round_state_info = next_account_info(account_info_iter)?;

        Ok(())
    }

    pub fn process_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        withdraw_params: WithdrawSolParams,
    ) -> ProgramResult {
        //todo be sure to test with more than 1 round
        let account_info_iter = &mut accounts.iter();
        let player_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let player_round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let player_token_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let WithdrawSolParams { withdraw_for_round } = withdraw_params;

        let (game_state, game_state_seed, game_state_bump) =
            deserialize_game_state(game_state_info, program_id)?;
        let round_state = deserialize_round_state(
            round_state_info,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        deserialize_pot(pot_info, withdraw_for_round, game_state.version, program_id)?;
        let mut player_round_state = deserialize_or_create_player_round_state(
            player_round_state_info,
            player_info,
            system_program_info,
            player_info.key,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        //ensure the user actually owns player_info
        if !player_info.is_signer {
            return Err(SomeError::BadError.into());
        }

        // --------------------------------------- calc withdrawal amounts
        let winnings_to_withdraw = if round_state.ended {
            player_round_state
                .accum_winnings
                .try_sub(player_round_state.withdrawn_winnings)?
        } else {
            0
        };
        let aff_to_withdraw = player_round_state
            .accum_aff
            .try_sub(player_round_state.withdrawn_aff)?;
        let f3d_to_withdraw = calculate_player_f3d_share(
            player_round_state.accum_keys,
            round_state.accum_keys,
            round_state.accum_f3d_share,
        )?;
        let total_to_withdraw = winnings_to_withdraw
            .try_add(aff_to_withdraw)?
            .try_add(f3d_to_withdraw)?;
        if total_to_withdraw == 0 {
            //should say something like "too little"
            return Err(SomeError::BadError.into());
        }

        // --------------------------------------- transfer tokens
        spl_token_transfer(TokenTransferParams {
            source: pot_info.clone(),
            destination: player_token_info.clone(),
            amount: total_to_withdraw.try_cast()?,
            authority: game_state_info.clone(),
            authority_signer_seeds: &[game_state_seed.as_bytes(), &[game_state_bump]],
            token_program: token_program_info.clone(),
        })?;

        // --------------------------------------- update player state
        player_round_state.withdrawn_aff += aff_to_withdraw;
        player_round_state.withdrawn_winnings += winnings_to_withdraw;
        player_round_state.withdrawn_f3d += f3d_to_withdraw;
        player_round_state.serialize(&mut *player_round_state_info.data.borrow_mut())?;

        Ok(())
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
        let player_pk = player_info.key;

        let (game_state, _, _) = deserialize_game_state(game_state_info, program_id)?;
        let mut round_state = deserialize_round_state(
            round_state_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        deserialize_pot(
            pot_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        let mut player_round_state = deserialize_or_create_player_round_state(
            player_round_state_info,
            player_info,
            system_program_info,
            player_pk,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        //ensure the user actually owns player_info
        if !player_info.is_signer {
            return Err(SomeError::BadError.into());
        }

        // --------------------------------------- calc & prep variables
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

        let fee_split;
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
            authority: player_info.clone(), //this also enforces player_info to be a signer
            token_program: token_program_info.clone(),
            amount: sol_to_be_added.try_cast()?,
            authority_signer_seeds: &[],
        })?;

        // --------------------------------------- part take in airdrop lottery
        //if they deposited > 0.1 sol, they're eligible for airdrop
        if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_floor_div(10)? {
            let clock = Clock::get()?;

            //with every extra player chance of airdrop increases by 0.1%
            round_state.airdrop_tracker += 1;

            if airdrop_winner(player_pk, &clock, round_state.airdrop_tracker)? {
                //NOTE: affiliate winnings _exclude_ contribution from this purchase, which is recorded below
                let airdrop_to_distribute = round_state.accum_airdrop_share;
                //3 tiers exist for airdrop
                let prize = if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_mul(10)? {
                    //10+ sol - win 75% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(75)?.try_floor_div(100)?
                } else if sol_to_be_added > LAMPORTS_PER_SOL as u128 {
                    //1-10 sol - win 50% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(50)?.try_floor_div(100)?
                } else {
                    //0.1-1 sol - win 25% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(25)?.try_floor_div(100)?
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
        let community_share = sol_to_be_added.try_floor_div(50)?;
        //1% to future airdrops
        let airdrop_share = sol_to_be_added.try_floor_div(100)?;
        //1% to next round's pot
        let next_round_share = sol_to_be_added.try_floor_div(100)?;
        //10% to affiliate
        let affiliate_share = sol_to_be_added.try_floor_div(10)?;

        //todo impl mechanism where only p3d member can withdraw
        let mut p3d_share = 0;
        let mut f3d_share = 0;

        //if player has an affiliate listed, record their share, else share goes to p3d holders
        if player_round_state.has_affiliate() {
            //optional account passed only if affiliate listed
            let affiliate_round_state_info = next_account_info(account_info_iter)?;
            let mut affiliate_round_state = deserialize_or_create_player_round_state(
                affiliate_round_state_info,
                player_info,
                system_program_info,
                &player_round_state.last_affiliate_pk,
                game_state.round_id,
                game_state.version,
                program_id,
            )?;
            affiliate_round_state.accum_aff += affiliate_share;
            round_state.accum_aff_share += affiliate_share;
        } else {
            p3d_share += affiliate_share;
        }

        p3d_share += sol_to_be_added
            .try_mul(fee_split.p3d as u128)?
            .try_floor_div(100)?;
        f3d_share += sol_to_be_added
            .try_mul(fee_split.f3d as u128)?
            .try_floor_div(100)?;

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

        verify_round_state(&round_state, pot_info)?;

        // --------------------------------------- serialize player-round state
        //update totals
        player_round_state.accum_keys += new_keys;
        player_round_state.accum_sol_added += sol_to_be_added;
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
        let rent_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        //todo need some sort of check that the previous round has ended to start a new round
        //todo also some sort of check on who can start the round

        let (mut game_state, _, _) = deserialize_game_state(game_state_info, program_id)?;
        game_state.round_id += 1;

        let mut round_state = create_round_state(
            round_state_info,
            funder_info,
            system_program_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        let pot = create_pot(
            pot_info,
            game_state_info,
            funder_info,
            mint_info,
            rent_info,
            system_program_info,
            token_program_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;

        //todo transfer money from previous round's pot, if such exists

        let clock = Clock::get()?;

        // all attributes not mentioned automatically start at 0.
        round_state.round_id = game_state.round_id;
        round_state.start_time = clock.unix_timestamp;
        round_state.end_time = round_state.start_time + ROUND_INIT_TIME;
        round_state.ended = false;
        round_state.accum_sol_pot = pot.amount as u128;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

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

        let mut game_state = create_game_state(
            game_state_info,
            funder_info,
            system_program_info,
            version,
            program_id,
        )?;

        //will be incremented to 1 when 1st round initialized
        game_state.round_id = 0;
        game_state.round_init_time = ROUND_INIT_TIME;
        game_state.round_inc_time = ROUND_INC_TIME;
        game_state.round_max_time = ROUND_MAX_TIME;
        game_state.version = version;
        game_state.serialize(&mut *game_state_info.data.borrow_mut())?;

        Ok(())
    }
}

//todo add rent checks
//todo add owner checks + other checks from eg token-lending
//todo https://blog.neodyme.io/posts/solana_common_pitfalls#solana-account-confusions