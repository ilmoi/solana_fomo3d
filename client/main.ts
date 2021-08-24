import {Token, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {
    AccountInfo,
    Connection, Keypair, LAMPORTS_PER_SOL,
    PublicKey, sendAndConfirmTransaction,
    Signer, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction,
    TransactionInstruction
} from "@solana/web3.js";
import BN from "bn.js";
import * as borsh from 'borsh';
import {assert} from "./utils";
import fs from "fs";
import {gameSchema, GameState} from "./layout";

// ============================================================================= globals & consts
let connection: Connection;
const FOMO_PROG_ID = new PublicKey("2HEMUe2d8HFfCMoBARcP5HSoKB5RRSg8dvLG4TVh2fHB");
const ownerKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([208, 175, 150, 242, 88, 34, 108, 88, 177, 16, 168, 75, 115, 181, 199, 242, 120, 4, 78, 75, 19, 227, 13, 215, 184, 108, 226, 53, 111, 149, 179, 84, 137, 121, 79, 1, 160, 223, 124, 241, 202, 203, 220, 237, 50, 242, 57, 158, 226, 207, 203, 188, 43, 28, 70, 110, 214, 234, 251, 15, 249, 157, 62, 80]));
const aliceKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([201, 101, 147, 128, 138, 189, 70, 190, 202, 49, 28, 26, 32, 21, 104, 185, 191, 41, 20, 171, 3, 144, 4, 26, 169, 73, 180, 171, 71, 22, 48, 135, 231, 91, 179, 215, 3, 117, 187, 183, 96, 74, 154, 155, 197, 243, 114, 104, 20, 123, 105, 47, 181, 123, 171, 133, 73, 181, 102, 41, 236, 78, 210, 176]));
const thirdPartyKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([177, 217, 193, 155, 63, 150, 164, 184, 81, 82, 121, 165, 202, 87, 86, 237, 218, 226, 212, 201, 167, 170, 149, 183, 59, 43, 155, 112, 189, 239, 231, 110, 162, 218, 184, 20, 108, 2, 92, 114, 203, 184, 223, 69, 137, 206, 102, 71, 162, 0, 127, 63, 170, 96, 137, 108, 228, 31, 181, 113, 57, 189, 30, 76]));

let gameState: PublicKey;
let roundState: PublicKey;
let aliceRoundState: PublicKey;

let wSolMint: Token;
let wSolAliceAcc: PublicKey;
let wSolComAcc: PublicKey;
let wSolP3dAcc: PublicKey;
let wSolPot: PublicKey;

let version: number;
//todo later need to sub round<1> for automatic
let round = 1;

// ============================================================================= helpers

function readAndUpdateVersion() {
    const contents = fs.readFileSync('version.txt');
    version = parseInt(contents.toString());
    console.log('running version', version);
    updateVersion();
}

function updateVersion() {
    fs.writeFileSync('version.txt', `${version + 1}`);
    console.log('file updated')
}

async function getConnection() {
    const url = 'http://localhost:8899';
    connection = new Connection(url, 'processed');
    const version = await connection.getVersion();
    console.log('connection to cluster established:', url, version);
}

async function prepareAndSendTx(instructions: TransactionInstruction[], signers: Signer[]) {
    const tx = new Transaction().add(...instructions);
    const sig = await sendAndConfirmTransaction(connection, tx, signers);
    console.log(sig);
}

async function generateCreateAccIx(newAccountPubkey: PublicKey, space: number): Promise<TransactionInstruction> {
    return SystemProgram.createAccount({
        programId: FOMO_PROG_ID,
        fromPubkey: ownerKp.publicKey,
        newAccountPubkey,
        space,
        lamports: await connection.getMinimumBalanceForRentExemption(space),
    });
}

async function createMintAccount(): Promise<Token> {
    return Token.createMint(
        connection,
        ownerKp,
        ownerKp.publicKey,
        null,
        0,
        TOKEN_PROGRAM_ID,
    );
}

async function createAndFundTokenAccount(mint: Token, owner: PublicKey, mintAmount: number = 0): Promise<PublicKey> {
    const tokenUserPk = await mint.createAccount(owner);
    if (mintAmount > 0) {
        await mint.mintTo(tokenUserPk, ownerKp.publicKey, [], mintAmount);
    }
    return tokenUserPk;
}

async function getGameState() {
    let gameStateInfo = await connection.getAccountInfo(gameState);
    let gameStateData = borsh.deserialize(gameSchema, GameState, gameStateInfo?.data as Buffer);
    console.log(gameStateData);
}

// ============================================================================= core

async function initGame() {
    console.log('// --------------------------------------- init game')
    //game state pda
    let stateBumpSeed;
    [gameState, stateBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`game${version}`)],
        FOMO_PROG_ID,
    )
    console.log('game state pda is:', gameState.toBase58());

    //configure all the token accounts
    wSolMint = await createMintAccount();
    wSolComAcc = await createAndFundTokenAccount(wSolMint, thirdPartyKp.publicKey);
    wSolP3dAcc = await createAndFundTokenAccount(wSolMint, thirdPartyKp.publicKey);
    wSolAliceAcc = await createAndFundTokenAccount(wSolMint, aliceKp.publicKey, 100 * LAMPORTS_PER_SOL);

    //init game ix
    const data = Buffer.from(Uint8Array.of(0, ...new BN(version).toArray('le', 1)));
    const initIx = new TransactionInstruction({
        keys: [
            {pubkey: ownerKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: gameState, isSigner: false, isWritable: true},
            {pubkey: wSolComAcc, isSigner: false, isWritable: false},
            {pubkey: wSolP3dAcc, isSigner: false, isWritable: false},
            {pubkey: wSolMint.publicKey, isSigner: false, isWritable: false},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
        ],
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([initIx], [ownerKp]);
    //todo unpack and verify game state
}

async function initRound(second = false) {
    console.log('// --------------------------------------- init round')
    let roundBumpSeed, potBumpSeed;
    //round state pda
    [roundState, roundBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`round${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('round state pda is:', roundState.toBase58());

    //pot pda
    [wSolPot, potBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`pot${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('round pot pda is:', wSolPot.toBase58());

    //keys
    let keys = [
        {pubkey: ownerKp.publicKey, isSigner: true, isWritable: false},
        {pubkey: gameState, isSigner: false, isWritable: true},
        {pubkey: roundState, isSigner: false, isWritable: true},
        {pubkey: wSolPot, isSigner: false, isWritable: true},
        {pubkey: wSolMint.publicKey, isSigner: false, isWritable: false},
        {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false
        },
        {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];
    if (second) {
        let [roundState2, roundBumpSeed2] = await PublicKey.findProgramAddress(
            [Buffer.from(`round${round - 1}${version}`)],
            FOMO_PROG_ID,
        )
        let [wSolPot2, potBumpSeed2] = await PublicKey.findProgramAddress(
            [Buffer.from(`pot${round - 1}${version}`)],
            FOMO_PROG_ID,
        )
        keys.push({pubkey: roundState2, isSigner: false, isWritable: true});
        keys.push({pubkey: wSolPot2, isSigner: false, isWritable: true});
    }

    //init round ix
    const data = Buffer.from(Uint8Array.of(1));
    const initRoundIx = new TransactionInstruction({
        keys,
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([initRoundIx], [ownerKp]);
    //todo unpack and verify round state

    let potAmount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('round pot has', potAmount as any / LAMPORTS_PER_SOL);
}

async function purchaseKeys(add_new_affiliate = false) {
    console.log('// --------------------------------------- purchase keys')
    let bump;
    //player-round state pda
    [aliceRoundState, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(`pr${aliceKp.publicKey.toBase58().substring(0, 16)}${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('player-round state pda is:', aliceRoundState.toBase58());

    //keys
    let keys = [
        {pubkey: aliceKp.publicKey, isSigner: true, isWritable: false},
        {pubkey: gameState, isSigner: false, isWritable: true},
        {pubkey: roundState, isSigner: false, isWritable: true},
        {pubkey: aliceRoundState, isSigner: false, isWritable: true},
        {pubkey: wSolPot, isSigner: false, isWritable: true},
        {pubkey: wSolAliceAcc, isSigner: false, isWritable: true},
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false
        },
        {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];
    if (add_new_affiliate) {
        let newAffPk = new PublicKey("59NC7XLBzG5knsnu51P9WDVtywvW64PnasV2piHepw13");
        let [newAffRoundState, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(`pr${newAffPk.toBase58().substring(0, 16)}${round}${version}`)],
        FOMO_PROG_ID,
    )
        keys.push({pubkey: newAffRoundState, isSigner: false, isWritable: true})
        keys.push({pubkey: newAffPk, isSigner: false, isWritable: false})
    }

    //init round ix
    const data = Buffer.from(Uint8Array.of(2,
        ...new BN(LAMPORTS_PER_SOL/2).toArray('le', 16), //1 sol
        ...new BN(1).toArray('le', 1), //team bear
    ));
    const purchaseKeysIx = new TransactionInstruction({
        keys,
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([purchaseKeysIx], [aliceKp]);
    //todo unpack and verify changes to game/round state

    let potAmount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('post purchase, pot has', potAmount as any / LAMPORTS_PER_SOL);
    let aliceAmount = (await connection.getTokenAccountBalance(wSolAliceAcc)).value.uiAmount;
    console.log('post purchase, alice has', aliceAmount as any / LAMPORTS_PER_SOL);
}

async function withdrawSol() {
    console.log('// --------------------------------------- withdraw sol')
    const data = Buffer.from(Uint8Array.of(3, ...new BN(round).toArray('le', 8)));
    const withdrawSolIx = new TransactionInstruction({
        keys: [
            {pubkey: aliceKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: false},
            {pubkey: aliceRoundState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolAliceAcc, isSigner: false, isWritable: true},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: FOMO_PROG_ID,
        data: data,
    });
    await prepareAndSendTx([withdrawSolIx], [aliceKp]);

    let potAmount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('post withdrawal, pot has', potAmount as any / LAMPORTS_PER_SOL);
    let aliceAmount = (await connection.getTokenAccountBalance(wSolAliceAcc)).value.uiAmount;
    console.log('post withdrawal, alice has', aliceAmount as any / LAMPORTS_PER_SOL);
}

async function endRound() {
    console.log('// --------------------------------------- end round')
    const data = Buffer.from(Uint8Array.of(4));
    const endRoundIx = new TransactionInstruction({
        keys: [
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: true},
            {pubkey: aliceRoundState, isSigner: false, isWritable: true},
        ],
        programId: FOMO_PROG_ID,
        data: data,
    });
    await prepareAndSendTx([endRoundIx], [ownerKp]);
}

async function withdrawCom() {
    console.log('// --------------------------------------- withdraw community funds')
    const data = Buffer.from(Uint8Array.of(5, ...new BN(round).toArray('le', 8)));
    const withdrawComIx = new TransactionInstruction({
        keys: [
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolComAcc, isSigner: false, isWritable: true},
            {pubkey: thirdPartyKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: FOMO_PROG_ID,
        data: data,
    });
    await prepareAndSendTx([withdrawComIx], [thirdPartyKp]);

    let potAmount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('post com withdrawal, pot has', potAmount as any / LAMPORTS_PER_SOL);
    let comAmount = (await connection.getTokenAccountBalance(wSolComAcc)).value.uiAmount;
    console.log('post com withdrawal, com account has', comAmount as any / LAMPORTS_PER_SOL);
}

// ============================================================================= play

async function play() {
    readAndUpdateVersion();
    await getConnection();
    await initGame();
    await initRound();
    await purchaseKeys();
    // await purchaseKeys(true);
    // await withdrawSol();
    // await setTimeout(async () => {
    //     await endRound();
    //     await withdrawSol();
    // }, 5000);
    // await withdrawCom();
    // round = 2;
    // await initRound(true);

}

play()

//todo remember to test the 2 places where you pass optional accounts
//todo basically try calling every function twice to see