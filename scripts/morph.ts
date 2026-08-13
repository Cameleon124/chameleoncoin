/**
 * Trigger a morph: burn 0.1% of supply from your wallet and rewrite the
 * token's name, symbol, and metadata URI.
 *
 * Usage:
 *   CHAMELEON_MINT=<mint address> \
 *   ts-node scripts/morph.ts "New Name" "TICK" "https://host.example/new-meta.json"
 */
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { PROGRAM_ID as TOKEN_METADATA_PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import { Chameleon } from "../target/types/chameleon";

async function main() {
  const [name, symbol, uri] = process.argv.slice(2);
  if (!name || !symbol || !uri) {
    console.error('Usage: ts-node scripts/morph.ts "Name" "SYMBOL" "https://.../meta.json"');
    process.exit(1);
  }
  const mintEnv = process.env.CHAMELEON_MINT;
  if (!mintEnv) throw new Error("Set CHAMELEON_MINT env var");
  const mint = new PublicKey(mintEnv);

  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Chameleon as Program<Chameleon>;
  const payer = provider.wallet.publicKey;

  const [metadataPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    TOKEN_METADATA_PROGRAM_ID
  );

  const sig = await program.methods
    .morph(name, symbol, uri)
    .accounts({
      payer,
      mint,
      payerTokenAccount: getAssociatedTokenAddressSync(mint, payer),
      metadata: metadataPda,
      tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
    })
    .rpc();

  console.log(`Morphed to "${name}" ($${symbol})`);
  console.log("Tx:", sig);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
