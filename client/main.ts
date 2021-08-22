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

// ============================================================================= globals & consts
let connection: Connection;
const OUR_PROGRAM_ID = new PublicKey("2HEMUe2d8HFfCMoBARcP5HSoKB5RRSg8dvLG4TVh2fHB");
const ownerKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([208, 175, 150, 242, 88, 34, 108, 88, 177, 16, 168, 75, 115, 181, 199, 242, 120, 4, 78, 75, 19, 227, 13, 215, 184, 108, 226, 53, 111, 149, 179, 84, 137, 121, 79, 1, 160, 223, 124, 241, 202, 203, 220, 237, 50, 242, 57, 158, 226, 207, 203, 188, 43, 28, 70, 110, 214, 234, 251, 15, 249, 157, 62, 80]));

let fomoState: PublicKey;

let wSolMint: Token;
let wSolUserAcc: PublicKey;
let wSolPot: PublicKey;

let randX = 24;

// ============================================================================= fns
// --------------------------------------- helpers

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

async function transferToken(from: PublicKey, to: PublicKey) {

}

// --------------------------------------- core

async function initFomo() {
    let stateBumpSeed, potBumpSeed;
    //state pda
    [fomoState, stateBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`state${randX}`)],
        OUR_PROGRAM_ID,
    )
    console.log('state pda is:', fomoState.toBase58());

    //pot pda
    [wSolPot, potBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`pot${randX}`)],
        OUR_PROGRAM_ID,
    )
    console.log('pot pda is:', wSolPot.toBase58());

    //wSol accounts
    wSolMint = await createMintAccount();
    wSolUserAcc = await createAndFundTokenAccount(wSolMint, ownerKp.publicKey, 10);

    //init ix
    const data = Buffer.from(Uint8Array.of(0, ...new BN(randX).toArray('le', 1)));
    const initIx = new TransactionInstruction({
        keys: [
            {pubkey: ownerKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: fomoState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolMint.publicKey, isSigner: false, isWritable: false},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: OUR_PROGRAM_ID,
        data,
    });
    await prepareAndSendTx([initIx], [ownerKp]);
}

async function sendAndGetBack() {
    //send there
    await wSolMint.transfer(
        wSolUserAcc,
        wSolPot,
        ownerKp,
        [],
        5
    );
    console.log('pot has', (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount);

    //send back
    const data2 = Buffer.from(Uint8Array.of(2, ...new BN(randX).toArray('le', 1)));
    const payOutIx = new TransactionInstruction({
        keys: [
            {pubkey: fomoState, isSigner: false, isWritable: false},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolUserAcc, isSigner: false, isWritable: true},
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: OUR_PROGRAM_ID,
        data: data2,
    });
    await prepareAndSendTx([payOutIx], [ownerKp]);
    console.log('pot has', (await connection.getTokenAccountBalance(wSolPot)).value.uiAmount);
}

// ============================================================================= play

async function play() {
    console.log('yay');
    await getConnection();
    await initFomo();
    await sendAndGetBack();
}

play()