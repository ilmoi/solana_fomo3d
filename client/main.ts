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
const OUR_PROGRAM_ID = new PublicKey("2HEMUe2d8HFfCMoBARcP5HSoKB5RRSg8dvLG4TVh2fHB");
const ownerKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([208, 175, 150, 242, 88, 34, 108, 88, 177, 16, 168, 75, 115, 181, 199, 242, 120, 4, 78, 75, 19, 227, 13, 215, 184, 108, 226, 53, 111, 149, 179, 84, 137, 121, 79, 1, 160, 223, 124, 241, 202, 203, 220, 237, 50, 242, 57, 158, 226, 207, 203, 188, 43, 28, 70, 110, 214, 234, 251, 15, 249, 157, 62, 80]));
const aliceKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([201, 101, 147, 128, 138, 189, 70, 190, 202, 49, 28, 26, 32, 21, 104, 185, 191, 41, 20, 171, 3, 144, 4, 26, 169, 73, 180, 171, 71, 22, 48, 135, 231, 91, 179, 215, 3, 117, 187, 183, 96, 74, 154, 155, 197, 243, 114, 104, 20, 123, 105, 47, 181, 123, 171, 133, 73, 181, 102, 41, 236, 78, 210, 176]));

let gameState: PublicKey;
let roundState: PublicKey;
let playerRoundState: PublicKey;

let wSolMint: Token;
let wSolAliceAcc: PublicKey;
let wSolPot: PublicKey;

let version: number;
//todo later need to sub round<1> for automatic
const round = 1;

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
        programId: OUR_PROGRAM_ID,
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
        OUR_PROGRAM_ID,
    )
    console.log('game state pda is:', gameState.toBase58());

    //init game ix
    const data = Buffer.from(Uint8Array.of(0, ...new BN(version).toArray('le', 1)));
    const initIx = new TransactionInstruction({
        keys: [
            {pubkey: ownerKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: gameState, isSigner: false, isWritable: true},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
        ],
        programId: OUR_PROGRAM_ID,
        data,
    });
    await prepareAndSendTx([initIx], [ownerKp]);
    //todo unpack and verify game state
}

async function initRound() {
    console.log('// --------------------------------------- init round')
    let roundBumpSeed, potBumpSeed;
    //round state pda
    [roundState, roundBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`round${round}${version}`)],
        OUR_PROGRAM_ID,
    )
    console.log('round state pda is:', roundState.toBase58());

    //pot pda
    [wSolPot, potBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`pot${round}${version}`)],
        OUR_PROGRAM_ID,
    )
    console.log('round pot pda is:', wSolPot.toBase58());

    //wSol accounts
    wSolMint = await createMintAccount();
    wSolAliceAcc = await createAndFundTokenAccount(wSolMint, aliceKp.publicKey, 100 * LAMPORTS_PER_SOL);

    //init round ix
    const data = Buffer.from(Uint8Array.of(1));
    const initRoundIx = new TransactionInstruction({
        keys: [
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
        ],
        programId: OUR_PROGRAM_ID,
        data,
    });
    await prepareAndSendTx([initRoundIx], [ownerKp]);
    //todo unpack and verify round state
}

async function purchaseKeys() {
    console.log('// --------------------------------------- purchase keys')
    let bump;
    //player-round state pda
    [playerRoundState, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(`pr${aliceKp.publicKey.toBase58().substring(0,16)}${round}${version}`)],
        OUR_PROGRAM_ID,
    )
    console.log('player-round state pda is:', playerRoundState.toBase58());

    //init round ix
    const data = Buffer.from(Uint8Array.of(2,
        ...new BN(LAMPORTS_PER_SOL).toArray('le', 16), //1 sol
        ...new BN(1).toArray('le', 1), //team bear
    ));
    const purchaseKeysIx = new TransactionInstruction({
        keys: [
            {pubkey: aliceKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: gameState, isSigner: false, isWritable: true},
            {pubkey: roundState, isSigner: false, isWritable: true},
            {pubkey: playerRoundState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolAliceAcc, isSigner: false, isWritable: true},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: OUR_PROGRAM_ID,
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
            {pubkey: playerRoundState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolAliceAcc, isSigner: false, isWritable: true},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: OUR_PROGRAM_ID,
        data: data,
    });
    await prepareAndSendTx([withdrawSolIx], [aliceKp]);

    let potAmount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('post withdrawal, pot has', potAmount as any / LAMPORTS_PER_SOL);
    let aliceAmount = (await connection.getTokenAccountBalance(wSolAliceAcc)).value.uiAmount;
    console.log('post withdrawal, alice has', aliceAmount as any / LAMPORTS_PER_SOL);
}

// ============================================================================= play

async function play() {
    readAndUpdateVersion();
    await getConnection();
    await initGame();
    await initRound();
    await purchaseKeys();
    await withdrawSol();
}

play()

