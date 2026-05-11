//! # locus-client
//!
//! Rust SDK for the Locus Anchor program + the ARMS HTTP service.
//!
//! Public API mirrors §5 of the build brief:
//! - `initialize_agent`, `commit_memory`, `update_read_fee`, `get_agent_memory`
//! - `query_with_attestation`, `query_unattested`, `fetch_attestation`

use std::sync::Arc;

use anchor_client::{Client, Cluster};
use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::Keypair, signature::Signature, signer::Signer,
};

pub use locus::{AgentMemory, RetrievalAttestation};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neighbor {
    pub id: String,
    pub coord: Vec<f32>,
    pub distance: f32,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AttestedQueryResult {
    pub neighbors: Vec<Neighbor>,
    pub attestation_signature: Signature,
    pub attestation_pda: Pubkey,
    pub query_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub version: u64,
    pub nonce: u64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct LocusClient {
    pub program_id: Pubkey,
    pub arms_endpoint: String,
    pub payer: Arc<Keypair>,
    rpc_url: String,
    ws_url: String,
    http: reqwest::Client,
}

impl LocusClient {
    pub fn new(
        rpc_url: &str,
        ws_url: Option<&str>,
        arms_endpoint: &str,
        program_id: Pubkey,
        payer: Keypair,
    ) -> Self {
        let ws = ws_url
            .map(|s| s.to_string())
            .unwrap_or_else(|| rpc_url.replacen("http", "ws", 1));
        Self {
            program_id,
            arms_endpoint: arms_endpoint.trim_end_matches('/').to_string(),
            payer: Arc::new(payer),
            rpc_url: rpc_url.to_string(),
            ws_url: ws,
            http: reqwest::Client::new(),
        }
    }

    pub fn payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub fn agent_pda(&self, owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"agent", owner.as_ref()], &self.program_id)
    }

    pub fn attestation_pda(&self, agent: &Pubkey, version: u64, nonce: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"attest",
                agent.as_ref(),
                &version.to_le_bytes(),
                &nonce.to_le_bytes(),
            ],
            &self.program_id,
        )
    }

    // -----------------------------------------------------------------------
    // Memory ops — owner = payer.
    // -----------------------------------------------------------------------

    pub async fn initialize_agent(
        &self,
        read_fee_lamports: u64,
        metadata_uri: &str,
    ) -> Result<Signature> {
        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());
        let owner = payer.pubkey();
        let (agent_pda, _) = self.agent_pda(&owner);
        let uri = metadata_uri.to_string();

        tokio::task::spawn_blocking(move || -> Result<Signature> {
            let anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            let sig = program
                .request()
                .accounts(locus::accounts::InitializeAgent {
                    agent_memory: agent_pda,
                    owner,
                    system_program: system_program::ID,
                })
                .args(locus::instruction::InitializeAgent {
                    read_fee_lamports,
                    metadata_uri: uri,
                })
                .signer(&*payer)
                .send()
                .map_err(|e| anyhow!("initialize_agent send: {e}"))?;
            Ok(sig)
        })
        .await
        .context("spawn_blocking initialize_agent")?
    }

    pub async fn commit_memory(&self, root: [u8; 32]) -> Result<Signature> {
        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());
        let owner = payer.pubkey();
        let (agent_pda, _) = self.agent_pda(&owner);

        tokio::task::spawn_blocking(move || -> Result<Signature> {
            let anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            let sig = program
                .request()
                .accounts(locus::accounts::CommitMemory {
                    agent_memory: agent_pda,
                    owner,
                })
                .args(locus::instruction::CommitMemory { root })
                .signer(&*payer)
                .send()
                .map_err(|e| anyhow!("commit_memory send: {e}"))?;
            Ok(sig)
        })
        .await
        .context("spawn_blocking commit_memory")?
    }

    pub async fn update_read_fee(&self, new_fee: u64) -> Result<Signature> {
        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());
        let owner = payer.pubkey();
        let (agent_pda, _) = self.agent_pda(&owner);

        tokio::task::spawn_blocking(move || -> Result<Signature> {
            let anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            let sig = program
                .request()
                .accounts(locus::accounts::UpdateReadFee {
                    agent_memory: agent_pda,
                    owner,
                })
                .args(locus::instruction::UpdateReadFee { new_fee })
                .signer(&*payer)
                .send()
                .map_err(|e| anyhow!("update_read_fee send: {e}"))?;
            Ok(sig)
        })
        .await
        .context("spawn_blocking update_read_fee")?
    }

    pub async fn get_agent_memory(&self, owner: &Pubkey) -> Result<AgentMemory> {
        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());
        let (agent_pda, _) = self.agent_pda(owner);

        tokio::task::spawn_blocking(move || -> Result<AgentMemory> {
            let anchor = Client::new_with_options(cluster, payer, CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            program
                .account::<AgentMemory>(agent_pda)
                .map_err(|e| anyhow!("fetch AgentMemory({agent_pda}): {e}"))
        })
        .await
        .context("spawn_blocking get_agent_memory")?
    }

    pub async fn fetch_attestation(&self, pda: Pubkey) -> Result<RetrievalAttestation> {
        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());

        tokio::task::spawn_blocking(move || -> Result<RetrievalAttestation> {
            let anchor = Client::new_with_options(cluster, payer, CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            program
                .account::<RetrievalAttestation>(pda)
                .map_err(|e| anyhow!("fetch RetrievalAttestation({pda}): {e}"))
        })
        .await
        .context("spawn_blocking fetch_attestation")?
    }

    // -----------------------------------------------------------------------
    // ARMS pass-through + attested retrieval
    // -----------------------------------------------------------------------

    pub async fn query_unattested(
        &self,
        _agent_owner: &Pubkey,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Neighbor>> {
        #[derive(Serialize)]
        struct Req<'a> {
            embedding: &'a [f32],
            k: usize,
        }
        #[derive(Deserialize)]
        struct Resp {
            neighbors: Vec<Neighbor>,
        }
        let resp: Resp = self
            .http
            .post(format!("{}/query", self.arms_endpoint))
            .json(&Req {
                embedding: query_embedding,
                k,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.neighbors)
    }

    /// 1) Hit ARMS `/query`.  2) Hash query + result.  3) Call `attest_retrieval`
    /// on Solana, paying `read_fee_lamports` from the requester to the owner.
    pub async fn query_with_attestation(
        &self,
        agent_owner: &Pubkey,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<AttestedQueryResult> {
        let agent = self.get_agent_memory(agent_owner).await?;
        let (agent_pda, _) = self.agent_pda(agent_owner);
        let version = agent.version;
        let nonce = generate_nonce();
        let (attestation_pda, _) = self.attestation_pda(&agent_pda, version, nonce);
        let owner = agent.owner;

        let neighbors = self.query_unattested(agent_owner, query_embedding, k).await?;
        let query_hash = hash_embedding(query_embedding);
        let result_hash = hash_neighbors(&neighbors);

        let program_id = self.program_id;
        let payer = self.payer.clone();
        let cluster = Cluster::Custom(self.rpc_url.clone(), self.ws_url.clone());

        let sig = tokio::task::spawn_blocking(move || -> Result<Signature> {
            let anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
            let program = anchor.program(program_id).map_err(|e| anyhow!("{e}"))?;
            let sig = program
                .request()
                .accounts(locus::accounts::AttestRetrieval {
                    agent_memory: agent_pda,
                    owner,
                    attestation: attestation_pda,
                    requester: payer.pubkey(),
                    system_program: system_program::ID,
                })
                .args(locus::instruction::AttestRetrieval {
                    query_hash,
                    result_hash,
                    nonce,
                })
                .signer(&*payer)
                .send()
                .map_err(|e| anyhow!("attest_retrieval send: {e}"))?;
            Ok(sig)
        })
        .await
        .context("spawn_blocking attest_retrieval")??;

        Ok(AttestedQueryResult {
            neighbors,
            attestation_signature: sig,
            attestation_pda,
            query_hash,
            result_hash,
            version,
            nonce,
        })
    }

}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

pub fn hash_embedding(embedding: &[f32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for x in embedding {
        hasher.update(x.to_le_bytes());
    }
    hasher.finalize().into()
}

pub fn hash_neighbors(neighbors: &[Neighbor]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for n in neighbors {
        hasher.update(n.id.as_bytes());
        hasher.update([0u8]);
        for x in &n.coord {
            hasher.update(x.to_le_bytes());
        }
        hasher.update(n.distance.to_le_bytes());
    }
    hasher.finalize().into()
}

fn generate_nonce() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0);
    // mix in a random low-bits salt so two queries in the same tick differ.
    let salt: u64 = rand_seed();
    micros.wrapping_mul(2654435761).wrapping_add(salt)
}

fn rand_seed() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::process::id().hash(&mut h);
    std::time::Instant::now().elapsed().as_nanos().hash(&mut h);
    h.finish()
}
