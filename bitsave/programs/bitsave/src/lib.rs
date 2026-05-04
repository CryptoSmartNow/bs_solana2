use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub mod constants;
pub mod errors;
pub mod state;

use crate::constants::*;
use crate::errors::*;
use crate::state::*;

declare_id!("3Tt5SpCSTEPseAaA9hSov5TCr8j6bksN4oWZ1x5y2321");

#[program]
pub mod bitsave {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        stable_coin_mint: Pubkey,
        cs_token_mint: Pubkey,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.set_inner(GlobalState {
            admin: ctx.accounts.admin.key(),
            stable_coin_mint,
            cs_token_mint,
            join_fee: 100_000,
            saving_fee: 100_000,
            total_value_locked: 0,
            user_count: 0,
        });
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

        user_vault.set_inner(UserVault {
            owner: ctx.accounts.user.key(),
        });
        global_state.user_count += 1;

        Ok(())
    }

    pub fn create_sol_saving(
        ctx: Context<CreateSolSaving>,
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

        let ix_fee = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &global_state.admin,
            global_state.saving_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix_fee,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let ix_transfer = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &user_vault.key(),
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix_transfer,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.user_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        saving.set_inner(Saving {
            owner: ctx.accounts.user.key(),
            name,
            amount,
            token_mint: Pubkey::default(),
            start_time: clock.unix_timestamp,
            maturity_time,
            penalty_percentage: penalty,
            is_safe_mode: safe_mode,
            is_valid: true,
        });

        global_state.total_value_locked += amount;

        Ok(())
    }

    pub fn create_token_saving(
        ctx: Context<CreateTokenSaving>,
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

        let ix_fee = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &global_state.admin,
            global_state.saving_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix_fee,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        saving.set_inner(Saving {
            owner: ctx.accounts.user.key(),
            name,
            amount,
            token_mint: ctx.accounts.token_mint.key(),
            start_time: clock.unix_timestamp,
            maturity_time,
            penalty_percentage: penalty,
            is_safe_mode: safe_mode,
            is_valid: true,
        });

        global_state.total_value_locked += amount;

        Ok(())
    }

    pub fn increment_sol_saving(ctx: Context<IncrementSolSaving>, amount: u64) -> Result<()> {
        let clock = Clock::get()?;
        let global_state = &mut ctx.accounts.global_state;
        let saving = &mut ctx.accounts.saving;
        let user_vault = &mut ctx.accounts.user_vault;

        if saving.maturity_time <= clock.unix_timestamp {
            return err!(BitsaveError::InvalidTime);
        }

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

        saving.amount += amount;
        global_state.total_value_locked += amount;

        Ok(())
    }

    pub fn increment_token_saving(ctx: Context<IncrementTokenSaving>, amount: u64) -> Result<()> {
        let clock = Clock::get()?;
        let global_state = &mut ctx.accounts.global_state;
        let saving = &mut ctx.accounts.saving;

        if saving.maturity_time <= clock.unix_timestamp {
            return err!(BitsaveError::InvalidTime);
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        saving.amount += amount;
        global_state.total_value_locked += amount;

        Ok(())
    }

    // withdraw SOL saving, apply penalty if withdrawn before maturity
    // close the saving account after withdrawal
    pub fn withdraw_sol_saving(ctx: Context<WithdrawSolSaving>) -> Result<()> {
        let clock = Clock::get()?;
        let saving = &mut ctx.accounts.saving;
        let global_state = &mut ctx.accounts.global_state;

        let mut amount_to_withdraw = saving.amount;

        if clock.unix_timestamp < saving.maturity_time {
            amount_to_withdraw =
                amount_to_withdraw * (100 - saving.penalty_percentage as u64) / 100;
        }

        **ctx
            .accounts
            .user_vault
            .to_account_info()
            .try_borrow_mut_lamports()? -= amount_to_withdraw;
        **ctx
            .accounts
            .user
            .to_account_info()
            .try_borrow_mut_lamports()? += amount_to_withdraw;

        global_state.total_value_locked -= saving.amount;
        // saving.is_valid = false;

        Ok(())
    }

    // withdraw token saving, apply penalty if withdrawn before maturity
    // close the saving account after withdrawal
    pub fn withdraw_token_saving(ctx: Context<WithdrawTokenSaving>) -> Result<()> {
        let clock = Clock::get()?;
        let saving = &mut ctx.accounts.saving;
        let global_state = &mut ctx.accounts.global_state;

        let mut amount_to_withdraw = saving.amount;

        if clock.unix_timestamp < saving.maturity_time {
            amount_to_withdraw =
                amount_to_withdraw * (100 - saving.penalty_percentage as u64) / 100;
        }

        let user_vault_seed = ctx.accounts.user.key();
        let seeds = &[
            USER_VAULT_SEED,
            user_vault_seed.as_ref(),
            &[ctx.bumps.user_vault],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.user_vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount_to_withdraw)?;

        global_state.total_value_locked -= saving.amount;
        // saving.is_valid = false;

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
    /// CHECK: The admin account specified in GlobalState. It receives the registration fees via System Program transfer.
    #[account(mut, address = global_state.admin)]
    pub admin_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateSolSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
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
    /// CHECK: The admin account specified in GlobalState.
    #[account(mut, address = global_state.admin)]
    pub admin_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateTokenSaving<'info> {
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
    /// CHECK: The admin account specified in GlobalState.
    #[account(mut, address = global_state.admin)]
    pub admin_account: AccountInfo<'info>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = token_mint, token::authority = user_vault)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct IncrementSolSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key() && saving.token_mint == Pubkey::default()
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct IncrementTokenSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key() && saving.token_mint != Pubkey::default()
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, token::mint = saving.token_mint, token::authority = user)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = saving.token_mint, token::authority = user_vault)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct WithdrawSolSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key() && saving.token_mint == Pubkey::default(),
        close = user
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTokenSaving<'info> {
    #[account(mut, seeds = [GLOBAL_STATE_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [USER_VAULT_SEED, user.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(
        mut,
        seeds = [SAVING_SEED, user_vault.key().as_ref(), saving.name.as_bytes()],
        bump,
        constraint = saving.is_valid && saving.owner == user.key() && saving.token_mint != Pubkey::default(),
        close = user
    )]
    pub saving: Account<'info, Saving>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, token::mint = saving.token_mint)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = saving.token_mint, token::authority = user_vault)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}
