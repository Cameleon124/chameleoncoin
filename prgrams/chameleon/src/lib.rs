use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{
        self,
        mpl_token_metadata::types::DataV2,
        Metadata, UpdateMetadataAccountsV2,
    },
    token::{self, Burn, Mint, Token, TokenAccount},
};

declare_id!("CHAMoRPH111111111111111111111111111111111111"); // replaced by `anchor keys sync`

/// Basis points burned per morph: 10 bps = 0.1% of current supply.
pub const MORPH_BURN_BPS: u64 = 10;
/// Optional cooldown between morphs, in seconds. 0 = disabled.
pub const MORPH_COOLDOWN_SECONDS: i64 = 0;

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 10;
pub const MAX_URI_LEN: usize = 200;

#[program]
pub mod chameleon {
    use super::*;

    /// One-time setup. Creates the config account that tracks the mint this
    /// program manages. The metadata update authority must be transferred to
    /// the `morph_authority` PDA off-chain (see scripts/initialize.ts) or via
    /// a prior UpdateMetadataAccountsV2 signed by the current authority.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.mint = ctx.accounts.mint.key();
        config.morph_count = 0;
        config.last_morph_ts = 0;
        config.bump = ctx.bumps.config;
        config.authority_bump = ctx.bumps.morph_authority;
        Ok(())
    }

    /// Burn 0.1% of the current supply from the caller, then rewrite the
    /// token's name, symbol, and metadata URI.
    pub fn morph(
        ctx: Context<Morph>,
        new_name: String,
        new_symbol: String,
        new_uri: String,
    ) -> Result<()> {
        require!(new_name.len() <= MAX_NAME_LEN, ChameleonError::NameTooLong);
        require!(
            new_symbol.len() <= MAX_SYMBOL_LEN,
            ChameleonError::SymbolTooLong
        );
        require!(new_uri.len() <= MAX_URI_LEN, ChameleonError::UriTooLong);
        require!(!new_name.trim().is_empty(), ChameleonError::EmptyName);
        require!(!new_symbol.trim().is_empty(), ChameleonError::EmptySymbol);

        let now = Clock::get()?.unix_timestamp;
        let config = &mut ctx.accounts.config;
        if MORPH_COOLDOWN_SECONDS > 0 {
            require!(
                now - config.last_morph_ts >= MORPH_COOLDOWN_SECONDS,
                ChameleonError::CooldownActive
            );
        }

        // 0.1% of *current* supply, floor division.
        let supply = ctx.accounts.mint.supply;
        let burn_amount = supply
            .checked_mul(MORPH_BURN_BPS)
            .ok_or(ChameleonError::MathOverflow)?
            / 10_000;
        require!(burn_amount > 0, ChameleonError::SupplyTooLow);
        require!(
            ctx.accounts.payer_token_account.amount >= burn_amount,
            ChameleonError::InsufficientBalance
        );

        // 1. Burn from the caller.
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    from: ctx.accounts.payer_token_account.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            burn_amount,
        )?;

        // 2. Update metadata, signed by the PDA update authority.
        let mint_key = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"morph_authority",
            mint_key.as_ref(),
            &[config.authority_bump],
        ]];

        metadata::update_metadata_accounts_v2(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                UpdateMetadataAccountsV2 {
                    metadata: ctx.accounts.metadata.to_account_info(),
                    update_authority: ctx.accounts.morph_authority.to_account_info(),
                },
                signer_seeds,
            ),
            None, // keep PDA as update authority forever
            Some(DataV2 {
                name: new_name.clone(),
                symbol: new_symbol.clone(),
                uri: new_uri.clone(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            }),
            None,       // primary_sale_happened unchanged
            Some(true), // remains mutable so it can morph again
        )?;

        config.morph_count = config
            .morph_count
            .checked_add(1)
            .ok_or(ChameleonError::MathOverflow)?;
        config.last_morph_ts = now;

        emit!(Morphed {
            morpher: ctx.accounts.payer.key(),
            mint: mint_key,
            new_name,
            new_symbol,
            new_uri,
            burned: burn_amount,
            morph_number: config.morph_count,
            timestamp: now,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config", mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    /// CHECK: PDA that will hold the metadata update authority. No data.
    #[account(
        seeds = [b"morph_authority", mint.key().as_ref()],
        bump
    )]
    pub morph_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Morph<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, address = config.mint)]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = payer,
    )]
    pub payer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"config", mint.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// CHECK: PDA update authority, verified by seeds.
    #[account(
        seeds = [b"morph_authority", mint.key().as_ref()],
        bump = config.authority_bump
    )]
    pub morph_authority: UncheckedAccount<'info>,

    /// CHECK: Metaplex metadata account for `mint`, verified by seeds against
    /// the token metadata program.
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key()
    )]
    pub metadata: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub mint: Pubkey,
    pub morph_count: u64,
    pub last_morph_ts: i64,
    pub bump: u8,
    pub authority_bump: u8,
}

#[event]
pub struct Morphed {
    pub morpher: Pubkey,
    pub mint: Pubkey,
    pub new_name: String,
    pub new_symbol: String,
    pub new_uri: String,
    pub burned: u64,
    pub morph_number: u64,
    pub timestamp: i64,
}

#[error_code]
pub enum ChameleonError {
    #[msg("Name exceeds 32 characters")]
    NameTooLong,
    #[msg("Symbol exceeds 10 characters")]
    SymbolTooLong,
    #[msg("URI exceeds 200 characters")]
    UriTooLong,
    #[msg("Name cannot be empty")]
    EmptyName,
    #[msg("Symbol cannot be empty")]
    EmptySymbol,
    #[msg("Caller does not hold enough tokens to burn 0.1% of supply")]
    InsufficientBalance,
    #[msg("Supply too low to compute a nonzero burn")]
    SupplyTooLow,
    #[msg("Morph cooldown still active")]
    CooldownActive,
    #[msg("Math overflow")]
    MathOverflow,
}
