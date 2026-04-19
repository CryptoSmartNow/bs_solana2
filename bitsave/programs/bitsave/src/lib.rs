use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};
use anchor_spl::associated_token::AssociatedToken;

pub mod constants;
pub mod state;
pub mod errors;

use crate::constants::*;
use crate::state::*;
use crate::errors::*;

declare_id!("8p4LcCZUsg53vjBP6F2cuWUuZNtHqX8v2EF72oQFkoLn");

#[program]
pub mod bitsave {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        stable_coin_mint: Pubkey,
        cs_token_mint: Pubkey,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin = ctx.accounts.admin.key();
        global_state.stable_coin_mint = stable_coin_mint;
        global_state.cs_token_mint = cs_token_mint;
        global_state.join_fee = 100_000;
        global_state.saving_fee = 100_000;
        global_state.total_value_locked = 0;
        global_state.user_count = 0;
        Ok(())
    }

    pub fn join_bitsave(ctx: Context<JoinBitsave>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        let user_vault = &mut ctx.accounts.user_vault;
        
        // Transfer join fee to admin
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &global_state.admin,
            global_state.join_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        user_vault.owner = ctx.accounts.user.key();
        global_state.user_count += 1;
        
        Ok(())
    }

    pub fn create_saving(
        ctx: Context<CreateSaving>,
        name: String,
        maturity_time: i64,
        penalty: u8,
        safe_mode: bool,
        amount: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        if maturity_time <= clock.unix_timestamp {
            return err!(BitsaveError::InvalidTime);
        }
        if safe_mode {
            return err!(BitsaveError::NotSupported);
        }

        let global_state = &mut ctx.accounts.global_state;
        let saving = &mut ctx.accounts.saving;
        let user_vault = &ctx.accounts.user_vault;

        // Transfer saving fee to admin
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &global_state.admin,
            global_state.saving_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Handle funds transfer (SOL or Token)
        if ctx.accounts.token_mint.key() == Pubkey::default() {
            // Transfer SOL to user_vault PDA
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.user.key(),
                &user_vault.key(),
                amount,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.user.to_account_info(),
                    ctx.accounts.user_vault.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        } else {
            // Transfer tokens to vault_token_account owned by user_vault PDA
            let cpi_accounts = Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
        }

        saving.owner = ctx.accounts.user.key();
        saving.name = name;
        saving.amount = amount;
        saving.token_mint = ctx.accounts.token_mint.key();
        saving.start_time = clock.unix_timestamp;
        saving.maturity_time = maturity_time;
        saving.penalty_percentage = penalty;
        saving.is_safe_mode = safe_mode;
        saving.is_valid = true;

        global_state.total_value_locked += amount;

        Ok(())
    }

    pub fn increment_saving(
        ctx: Context<IncrementSaving>,
        amount: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let global_state = &mut ctx.accounts.global_state;
        let saving = &mut ctx.accounts.saving;
        let user_vault = &mut ctx.accounts.user_vault;

        if saving.maturity_time <= clock.unix_timestamp {
            return err!(BitsaveError::InvalidTime);
        }

        // Handle funds transfer
        if saving.token_mint == Pubkey::default() {
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.user.key(),
                &user_vault.key(),
                amount,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.user.to_account_info(),
                    ctx.accounts.user_vault.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        } else {
            let cpi_accounts = Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
        }

        saving.amount += amount;
        global_state.total_value_locked += amount;

        Ok(())
    }

    pub fn withdraw_saving(ctx: Context<WithdrawSaving>) -> Result<()> {
        let clock = Clock::get()?;
        let saving = &mut ctx.accounts.saving;
        let global_state = &mut ctx.accounts.global_state;

        let mut amount_to_withdraw = saving.amount;

        if clock.unix_timestamp < saving.maturity_time {
            // Apply penalty
            amount_to_withdraw = amount_to_withdraw * (100 - saving.penalty_percentage as u64) / 100;
        }

        // Perform transfer from PDA to user
        let user_vault_seed = ctx.accounts.user.key();
        let seeds = &[
            USER_VAULT_SEED,
            user_vault_seed.as_ref(),
            &[ctx.bumps.user_vault],
        ];
        let signer = &[&seeds[..]];

        if saving.token_mint == Pubkey::default() {
            // Withdraw SOL from PDA
            **ctx.accounts.user_vault.to_account_info().try_borrow_mut_lamports()? -= amount_to_withdraw;
            **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += amount_to_withdraw;
        } else {
            // Withdraw tokens from vault_token_account owned by user_vault PDA
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.user_vault.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, amount_to_withdraw)?;
        }

        global_state.total_value_locked -= saving.amount;
        saving.is_valid = false;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + GlobalState::INIT_SPACE,
        seeds = [GLOBAL_STATE_SEED],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinBitsave<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        init,
        payer = user,
        space = 8 + UserVault::INIT_SPACE,
        seeds = [USER_VAULT_SEED, user.key().as_ref()],
        bump
    )]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Admin account to receive fees
    #[account(mut, address = global_state.admin)]
    pub admin_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        init,
        payer = user,
        space = 8 + Saving::INIT_SPACE,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), name.as_bytes()],
        bump
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Admin account to receive fees
    #[account(mut, address = global_state.admin)]
    pub admin_account: AccountInfo<'info>,
    /// CHECK: Token mint or Pubkey::default for SOL
    pub token_mint: AccountInfo<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub user_token_account: UncheckedAccount<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub vault_token_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct IncrementSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key()
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub user_token_account: UncheckedAccount<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub vault_token_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key(),
        close = user
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub user_token_account: UncheckedAccount<'info>,
    /// CHECK: Validated manually or via CPI
    #[account(mut)]
    pub vault_token_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
