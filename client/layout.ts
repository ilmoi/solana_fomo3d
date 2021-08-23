// todo intentionally pausing, otherwise too much work changing as program evolves

import BN from "bn.js";

// --------------------------------------- game state

//these have to match rust (ie no snake case)
interface IGameState {
    round_id: BN,
    round_init_time: BN,
    round_inc_time: BN,
    round_max_time: BN,
    version: number,
}

export class GameState {
    //defaults are to be able to calc size programmatically
    roundId = new BN(0);
    roundInitTime = new BN(0);
    roundIncTime = new BN(0);
    roundMaxTime = new BN(0);
    version = 0;

    constructor(fields?: IGameState) {
        if (fields) {
            this.roundId = fields.round_id;
            this.roundInitTime = fields.round_init_time;
            this.roundIncTime = fields.round_inc_time;
            this.roundMaxTime = fields.round_max_time;
            this.version = fields.version;
        }
    }
}

export const gameSchema = new Map([[GameState, {
    kind: 'struct',
    fields: [
        ['round_id', 'u64'],
        //todo borsh doesn't understand i64 - only u64 - see if this is a problem later
        ['round_init_time', 'u64'],
        ['round_inc_time', 'u64'],
        ['round_max_time', 'u64'],
        ['version', 'u8'],
    ]
}]])

// --------------------------------------- round state

interface IRoundState {
    round_id: BN,
    lead_player_pk: number[],
    lead_player_team: number,
    start_time: BN,
    end_time: BN,
    ended: number,
    accum_keys: BN,
    accum_sol_pot: BN,
    accum_f3d_share: BN,
    accum_p3d_share: BN,
    accum_community_share: BN,
    accum_next_round_share: BN,
    accum_airdrop_share: BN,
    airdrop_tracker: BN,
}

export class RoundState {
    roundId = new BN(0);
    leadPlayerPk = new Array(32);
    leadPlayerTeam = 0;
    startTime = new BN(0);
    endTime = new BN(0);
    ended = 0;
    accumKeys = new BN(0);
    accumSolPot = new BN(0);
    accumF3dShare = new BN(0);
    accumP3dShare = new BN(0);
    accumCommunityShare = new BN(0);
    accumNextRoundShare = new BN(0);
    accumAirdropShare = new BN(0);
    airdropTracker = new BN(0);

    constructor(fields?: IRoundState) {
        if (fields) {
            this.roundId = fields.round_id;
            this.leadPlayerPk = fields.lead_player_pk;
            this.leadPlayerTeam = fields.lead_player_team;
            this.startTime = fields.start_time;
            this.endTime = fields.end_time;
            this.ended = fields.ended;
            this.accumKeys = fields.accum_keys;
            this.accumSolPot = fields.accum_sol_pot;
            this.accumF3dShare = fields.accum_f3d_share;
            this.accumP3dShare = fields.accum_p3d_share;
            this.accumCommunityShare = fields.accum_community_share;
            this.accumNextRoundShare = fields.accum_next_round_share;
            this.accumAirdropShare = fields.accum_airdrop_share;
            this.airdropTracker = fields.airdrop_tracker;
        }
    }
}