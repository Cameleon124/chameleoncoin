# 🦎 Chameleon Coin

**The token that changes its skin.** Anyone can rename the token, change its ticker, and swap its image — by burning **0.1% of the current supply**.

Every "morph" is deflationary. The more the community fights over the name, the scarcer the token gets.

## How it works

1. The token's Metaplex **metadata update authority** is held by a program PDA (`morph_authority`).
2. Anyone holding tokens calls the `morph` instruction with a new `name`, `symbol`, and metadata `uri`.
3. The program:
   - Calculates `burn_amount = current_supply / 1000` (0.1%)
   - Burns that amount from the caller's token account
   - CPIs into the Metaplex Token Metadata program to update name/symbol/uri
   - Emits a `Morphed` event with the old→new identity and the amount burned

No admin keys. No backend. Fully on-chain.

## ⚠️ Important: pump.fun compatibility

Tokens launched **directly on pump.fun cannot be morphed** — pump.fun revokes the mint authority *and* sets the metadata update authority to immutable at launch. There is no way to change the name/ticker/image of a standard pump.fun token afterward.

Two realistic paths:

| Path | How |
|---|---|
| **Own launch (recommended)** | Mint your own SPL token with Metaplex metadata, transfer the update authority to this program's PDA (`scripts/initialize.ts` does this), then seed liquidity on Raydium/Meteora. Full morph functionality. |
| **pump.fun "wrapper" branding** | Launch on pump.fun for the bonding curve, but point the metadata `uri` (set once, before launch) at a JSON file you host whose *image* your community can vote to change. Name/ticker stay fixed — only off-chain image swaps possible, and only if you keep control of the URI host. Not trustless. |

This repo implements the fully on-chain version (path 1).

## Repo layout

```
programs/chameleon/     Anchor program (Rust)
scripts/initialize.ts   Create mint + metadata, hand update authority to the PDA
scripts/morph.ts        CLI to trigger a morph
tests/chameleon.ts      Anchor tests (localnet)
app/                    (optional) drop a frontend here
```

## Quick start

Prereqs: Rust, Solana CLI ≥ 1.18, Anchor ≥ 0.30, Node ≥ 18.

```bash
git clone <your-repo-url> && cd chameleon-coin
yarn install
anchor build
anchor keys sync          # writes the real program ID into lib.rs / Anchor.toml
anchor build              # rebuild with correct ID

# local test
anchor test

# devnet deploy
anchor deploy --provider.cluster devnet
ts-node scripts/initialize.ts   # creates mint, metadata, transfers update authority to PDA
ts-node scripts/morph.ts "New Name" "TICK" "https://example.com/meta.json"
```

## Morph rules (on-chain enforced)

- Burn amount: `supply / 10000`, recalculated at call time — burns shrink as supply shrinks
- Caller must hold ≥ the burn amount
- `name` ≤ 32 chars, `symbol` ≤ 10 chars, `uri` ≤ 200 chars (Metaplex limits)
- Optional cooldown between morphs (`MORPH_COOLDOWN_SECONDS` in `lib.rs`, default 0 = disabled)

## Security notes

- The PDA is the *only* update authority; the program never exposes an instruction to transfer it back, so the metadata can never be rugged to a private key.
- Mint authority should be burned/revoked after initial mint (`initialize.ts` revokes it).
- Nothing here is financial advice; audit before mainnet.

## License

MIT
