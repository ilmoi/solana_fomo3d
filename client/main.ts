import {Token, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {
    AccountInfo,
    Connection, Keypair,
    PublicKey, sendAndConfirmTransaction,
    Signer, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction,
    TransactionInstruction
} from "@solana/web3.js";
import BN from "bn.js";
import * as borsh from 'borsh';
import {assert} from "./utils";
import fs from "fs";

// ============================================================================= globals & consts
let connection: Connection;
const OUR_PROGRAM_ID = new PublicKey("2HEMUe2d8HFfCMoBARcP5HSoKB5RRSg8dvLG4TVh2fHB");
const ownerKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([208, 175, 150, 242, 88, 34, 108, 88, 177, 16, 168, 75, 115, 181, 199, 242, 120, 4, 78, 75, 19, 227, 13, 215, 184, 108, 226, 53, 111, 149, 179, 84, 137, 121, 79, 1, 160, 223, 124, 241, 202, 203, 220, 237, 50, 242, 57, 158, 226, 207, 203, 188, 43, 28, 70, 110, 214, 234, 251, 15, 249, 157, 62, 80]));

let gameState: PublicKey;
let roundState: PublicKey;

let wSolMint: Token;
let wSolUserAcc: PublicKey;
let wSolPot: PublicKey;

let version: number;

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
}

async function initRound() {
    //todo later need to sub round<1> for automatic
    const round = 1;

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
    wSolUserAcc = await createAndFundTokenAccount(wSolMint, ownerKp.publicKey, 10);

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
}

async function sendAndGetBack() {
    console.log('// --------------------------------------- send and back')
    let amount;
    //send there
    await wSolMint.transfer(
        wSolUserAcc,
        wSolPot,
        ownerKp,
        [],
        5
    );
    amount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('pot has', amount);
    assert(amount == 5);

    //send back
    const data = Buffer.from(Uint8Array.of(3));
    const payOutIx = new TransactionInstruction({
        keys: [
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolUserAcc, isSigner: false, isWritable: true},
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: OUR_PROGRAM_ID,
        data: data,
    });
    await prepareAndSendTx([payOutIx], [ownerKp]);
    amount = (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount;
    console.log('pot has', amount);
    assert(amount == 4);
}

// ============================================================================= play

async function play() {
    readAndUpdateVersion();
    await getConnection();
    await initGame();
    await initRound();
    await sendAndGetBack();
}

play()