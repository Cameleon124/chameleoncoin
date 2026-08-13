/**
 * Creates the Chameleon mint, mints initial supply to your wallet, creates
 * Metaplex metadata, transfers the metadata update authority to the program's
 * morph_authority PDA, revokes the mint authority, and calls `initialize`.
 *
 * Usage:
 *   ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
 *   ANCHOR_WALLET=~/.config/solana/id.json \
 *   ts-node scripts/initialize.ts
 */
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  createInitializeMintInstruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createSetAuthorityInstruction,
  getAssociatedTokenAddressSync,
  AuthorityType,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  getMinimumBalanceForRentExemptMint,
} from "@solana/spl-token";
import {
  createCreateMetadataAccountV3Instruction,
  createUpdateMetadataAccountV2Instruction,
  PROGRAM_ID as TOKEN_METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { Chameleon } from "../target/types/chameleon";

// ---- Launch configuration ---------------------------------------------
const INITIAL_NAME = "Chameleon";
const INITIAL_SYMBOL = "CHAM";
const INITIAL_URI = "https://your-host.example/chameleon.json"; // JSON with image field
const DECIMALS = 6;
const INITIAL_SUPPLY = 1_000_000_000n * 10n ** BigInt(DECIMALS); // 1B tokens
// ------------------------------------------------------------------------

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Chameleon as Program<Chameleon>;
  const payer = provider.wallet.publicKey;

  const mintKp = Keypair.generate();
  const mint = mintKp.publicKey;
  console.log("Mint:", mint.toBase58());

  const [morphAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("morph_authority"), mint.toBuffer()],
    program.programId
  );
  const [metadataPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    TOKEN_METADATA_PROGRAM_ID
  );
  const ata = getAssociatedTokenAddressSync(mint, payer);
  const rent = await getMinimumBalanceForRentExemptMint(provider.connection);

  const tx = new Transaction().add(
    // 1. Create + init mint
    SystemProgram.createAccount({
      fromPubkey: payer,
      newAccountPubkey: mint,
      space: MINT_SIZE,
      lamports: rent,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeMintInstruction(mint, DECIMALS, payer, null),
    // 2. Mint initial supply to your ATA
    createAssociatedTokenAccountInstruction(payer, ata, payer, mint),
    createMintToInstruction(mint, ata, payer, INITIAL_SUPPLY),
    // 3. Create metadata (payer as initial update authority)
    createCreateMetadataAccountV3Instruction(
      {
        metadata: metadataPda,
        mint,
        mintAuthority: payer,
        payer,
        updateAuthority: payer,
      },
      {
        createMetadataAccountArgsV3: {
          data: {
            name: INITIAL_NAME,
            symbol: INITIAL_SYMBOL,
            uri: INITIAL_URI,
            sellerFeeBasisPoints: 0,
            creators: null,
            collection: null,
            uses: null,
          },
          isMutable: true,
          collectionDetails: null,
        },
      }
    ),
    // 4. Hand the update authority to the program PDA — irreversible by design
    createUpdateMetadataAccountV2Instruction(
      { metadata: metadataPda, updateAuthority: payer },
      {
        updateMetadataAccountArgsV2: {
          data: null,
          updateAuthority: morphAuthority,
          primarySaleHappened: null,
          isMutable: true,
        },
      }
    ),
    // 5. Revoke mint authority so supply can only ever go down
    createSetAuthorityInstruction(mint, payer, AuthorityType.MintTokens, null)
  );

  const sig = await provider.sendAndConfirm(tx, [mintKp]);
  console.log("Setup tx:", sig);

  // 6. Initialize program config
  const initSig = await program.methods
    .initialize()
    .accounts({ payer, mint })
    .rpc();
  console.log("Initialize tx:", initSig);
  console.log("Morph authority PDA:", morphAuthority.toBase58());
  console.log("Done. The token can now be morphed by anyone burning 0.1%.");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
