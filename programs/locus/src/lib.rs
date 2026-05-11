//! # Locus — Solana protocol for verifiable AI memory
//!
//! Locus anchors ARMS (Attention Reasoning Memory Store) memory roots on
//! Solana and produces cryptographic attestations for every paid retrieval.
//!
//! ## Accounts
//! - `AgentMemory`  — per-agent PDA holding the current memory root + fees.
//! - `RetrievalAttestation` — per-retrieval PDA recording query / result hashes.
//!
//! ## Instructions
//! - `initialize_agent` — create `AgentMemory` PDA (owner pays rent).
//! - `commit_memory`    — owner-only; updates root + version.
//! - `attest_retrieval` — anyone; pays read fee; creates attestation PDA.
//! - `update_read_fee`  — owner-only.
//! - `close_agent`      — owner-only; refunds rent.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("C6AJ43ZpzPLtmcwDS1FQP7cQXtWHNwsLty5ijdLTxzmK");

pub const MAX_METADATA_URI_LEN: usize = 200;

#[program]
pub mod locus {
    use super::*;

    pub fn initialize_agent(
        ctx: Context<InitializeAgent>,
        read_fee_lamports: u64,
        metadata_uri: String,
    ) -> Result<()> {
        require!(
            metadata_uri.len() <= MAX_METADATA_URI_LEN,
            LocusError::MetadataUriTooLong
        );

        let agent = &mut ctx.accounts.agent_memory;
        agent.owner = ctx.accounts.owner.key();
        agent.memory_root = [0u8; 32];
        agent.version = 0;
        agent.last_updated = Clock::get()?.unix_timestamp;
        agent.read_fee_lamports = read_fee_lamports;
        agent.write_count = 0;
        agent.read_count = 0;
        agent.metadata_uri = metadata_uri;
        agent.bump = ctx.bumps.agent_memory;
        Ok(())
    }

    pub fn commit_memory(ctx: Context<CommitMemory>, root: [u8; 32]) -> Result<()> {
        require!(root != [0u8; 32], LocusError::InvalidRoot);

        let agent = &mut ctx.accounts.agent_memory;
        let now = Clock::get()?.unix_timestamp;

        agent.memory_root = root;
        agent.version = agent.version.checked_add(1).unwrap();
        agent.last_updated = now;
        agent.write_count = agent.write_count.checked_add(1).unwrap();

        emit!(MemoryCommitted {
            agent: agent.key(),
            version: agent.version,
            root,
            timestamp: now,
        });
        Ok(())
    }

    pub fn attest_retrieval(
        ctx: Context<AttestRetrieval>,
        query_hash: [u8; 32],
        result_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let agent_key = ctx.accounts.agent_memory.key();
        let agent = &mut ctx.accounts.agent_memory;
        let now = Clock::get()?.unix_timestamp;
        let fee = agent.read_fee_lamports;

        if fee > 0 {
            // Transfer fee from requester -> owner via System Program CPI.
            let ix = system_instruction::transfer(
                &ctx.accounts.requester.key(),
                &ctx.accounts.owner.key(),
                fee,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.requester.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )
            .map_err(|_| LocusError::InsufficientPayment)?;
        }

        agent.read_count = agent.read_count.checked_add(1).unwrap();

        let attestation = &mut ctx.accounts.attestation;
        attestation.agent = agent_key;
        attestation.memory_root = agent.memory_root;
        attestation.version = agent.version;
        attestation.query_hash = query_hash;
        attestation.result_hash = result_hash;
        attestation.requester = ctx.accounts.requester.key();
        attestation.timestamp = now;
        attestation.nonce = nonce;
        attestation.bump = ctx.bumps.attestation;

        emit!(RetrievalAttested {
            agent: agent_key,
            version: agent.version,
            query_hash,
            result_hash,
            requester: ctx.accounts.requester.key(),
            timestamp: now,
        });
        Ok(())
    }

    pub fn update_read_fee(ctx: Context<UpdateReadFee>, new_fee: u64) -> Result<()> {
        ctx.accounts.agent_memory.read_fee_lamports = new_fee;
        Ok(())
    }

    pub fn close_agent(_ctx: Context<CloseAgent>) -> Result<()> {
        // Anchor `close = owner` constraint refunds rent automatically.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[account]
pub struct AgentMemory {
    pub owner: Pubkey,
    pub memory_root: [u8; 32],
    pub version: u64,
    pub last_updated: i64,
    pub read_fee_lamports: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub metadata_uri: String,
    pub bump: u8,
}

impl AgentMemory {
    // 8 disc + 32 + 32 + 8 + 8 + 8 + 8 + 8 + (4 + MAX_URI) + 1
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 4 + MAX_METADATA_URI_LEN + 1;
}

#[account]
pub struct RetrievalAttestation {
    pub agent: Pubkey,
    pub memory_root: [u8; 32],
    pub version: u64,
    pub query_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub requester: Pubkey,
    pub timestamp: i64,
    pub nonce: u64,
    pub bump: u8,
}

impl RetrievalAttestation {
    // 8 disc + 32 + 32 + 8 + 32 + 32 + 32 + 8 + 8 + 1
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 32 + 32 + 32 + 8 + 8 + 1;
}

// ---------------------------------------------------------------------------
// Contexts
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeAgent<'info> {
    #[account(
        init,
        payer = owner,
        space = AgentMemory::SPACE,
        seeds = [b"agent", owner.key().as_ref()],
        bump,
    )]
    pub agent_memory: Account<'info, AgentMemory>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CommitMemory<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_memory.bump,
        has_one = owner @ LocusError::Unauthorized,
    )]
    pub agent_memory: Account<'info, AgentMemory>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(query_hash: [u8;32], result_hash: [u8;32], nonce: u64)]
pub struct AttestRetrieval<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_memory.bump,
        has_one = owner @ LocusError::Unauthorized,
    )]
    pub agent_memory: Account<'info, AgentMemory>,

    /// CHECK: validated via `has_one = owner` on agent_memory.
    #[account(mut)]
    pub owner: AccountInfo<'info>,

    #[account(
        init,
        payer = requester,
        space = RetrievalAttestation::SPACE,
        seeds = [
            b"attest",
            agent_memory.key().as_ref(),
            &agent_memory.version.to_le_bytes(),
            &nonce.to_le_bytes(),
        ],
        bump,
    )]
    pub attestation: Account<'info, RetrievalAttestation>,

    #[account(mut)]
    pub requester: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateReadFee<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_memory.bump,
        has_one = owner @ LocusError::Unauthorized,
    )]
    pub agent_memory: Account<'info, AgentMemory>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAgent<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_memory.bump,
        has_one = owner @ LocusError::Unauthorized,
        close = owner,
    )]
    pub agent_memory: Account<'info, AgentMemory>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[event]
pub struct MemoryCommitted {
    pub agent: Pubkey,
    pub version: u64,
    pub root: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct RetrievalAttested {
    pub agent: Pubkey,
    pub version: u64,
    pub query_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub requester: Pubkey,
    pub timestamp: i64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[error_code]
pub enum LocusError {
    #[msg("Caller is not the agent owner.")]
    Unauthorized,
    #[msg("Requester did not provide sufficient SOL for the read fee.")]
    InsufficientPayment,
    #[msg("Metadata URI exceeds 200 chars.")]
    MetadataUriTooLong,
    #[msg("Memory root must not be all zeros.")]
    InvalidRoot,
}
