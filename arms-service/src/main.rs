//! # arms-service
//!
//! Thin HTTP wrapper around the `arms` crate. Locus components talk to ARMS
//! through this service rather than linking the crate directly — keeps ARMS
//! itself untouched (per build-brief §12).
//!
//! Routes:
//!   POST /place        { id, coord: [f32], payload? }       -> { id }
//!   POST /get          { id }                                -> { coord, payload }
//!   POST /query        { embedding: [f32], k }               -> { neighbors: [...] }
//!   GET  /state-root                                          -> { root: "<hex>", count }
//!   GET  /healthz                                             -> "ok"

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use arms::{Arms, ArmsConfig, Point};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: SocketAddr,
    /// ARMS dimensionality.
    #[arg(long, default_value_t = 64)]
    dim: usize,
}

#[derive(Clone)]
struct AppState {
    arms: Arc<RwLock<Arms>>,
    // id -> (coord, payload) — kept alongside ARMS so `/get` can return the
    // raw embedding without poking core internals.
    index: Arc<RwLock<BTreeMap<String, (Vec<f32>, serde_json::Value)>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "arms_service=info".into()),
        )
        .init();

    let args = Args::parse();
    let arms = Arms::new(ArmsConfig::new(args.dim));
    let state = AppState {
        arms: Arc::new(RwLock::new(arms)),
        index: Arc::new(RwLock::new(BTreeMap::new())),
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/place", post(place))
        .route("/get", post(get_one))
        .route("/query", post(query))
        .route("/state-root", get(state_root))
        .with_state(state);

    info!(addr = %args.bind, dim = args.dim, "arms-service listening");
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Deserialize)]
struct PlaceReq {
    id: String,
    coord: Vec<f32>,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct PlaceResp {
    id: String,
}

async fn place(State(s): State<AppState>, Json(req): Json<PlaceReq>) -> Json<PlaceResp> {
    let point = Point::new(req.coord.clone());
    let payload_bytes = serde_json::to_vec(&req.payload).unwrap_or_default();
    let mut arms = s.arms.write().await;
    let _ = arms.place(point, payload_bytes.into());
    drop(arms);
    s.index
        .write()
        .await
        .insert(req.id.clone(), (req.coord, req.payload));
    Json(PlaceResp { id: req.id })
}

#[derive(Deserialize)]
struct GetReq {
    id: String,
}

#[derive(Serialize)]
struct GetResp {
    coord: Vec<f32>,
    payload: serde_json::Value,
}

async fn get_one(
    State(s): State<AppState>,
    Json(req): Json<GetReq>,
) -> Result<Json<GetResp>, (axum::http::StatusCode, String)> {
    let idx = s.index.read().await;
    let (coord, payload) = idx
        .get(&req.id)
        .cloned()
        .ok_or((axum::http::StatusCode::NOT_FOUND, format!("{} not found", req.id)))?;
    Ok(Json(GetResp { coord, payload }))
}

#[derive(Deserialize)]
struct QueryReq {
    embedding: Vec<f32>,
    k: usize,
}

#[derive(Serialize, Clone)]
struct Neighbor {
    id: String,
    coord: Vec<f32>,
    distance: f32,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct QueryResp {
    neighbors: Vec<Neighbor>,
}

async fn query(State(s): State<AppState>, Json(req): Json<QueryReq>) -> Json<QueryResp> {
    // Brute-force over the in-memory index for the demo. Once arms-core's
    // `near()` returns ids + distances we can swap this out.
    let idx = s.index.read().await;
    let mut scored: Vec<Neighbor> = idx
        .iter()
        .map(|(id, (coord, payload))| {
            let d = euclid(&req.embedding, coord);
            Neighbor {
                id: id.clone(),
                coord: coord.clone(),
                distance: d,
                payload: payload.clone(),
            }
        })
        .collect();
    scored.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(req.k);
    Json(QueryResp { neighbors: scored })
}

#[derive(Serialize)]
struct RootResp {
    root: String,
    count: usize,
}

async fn state_root(State(s): State<AppState>) -> Json<RootResp> {
    let idx = s.index.read().await;
    let mut leaves: Vec<[u8; 32]> = idx
        .iter()
        .map(|(id, (coord, _))| leaf_hash(id, coord))
        .collect();
    leaves.sort_unstable();
    let root = merkle(&leaves);
    Json(RootResp {
        root: hex::encode(root),
        count: leaves.len(),
    })
}

fn euclid(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn leaf_hash(id: &str, coord: &[f32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(id.as_bytes());
    h.update([0u8]);
    for x in coord {
        h.update(x.to_le_bytes());
    }
    h.finalize().into()
}

fn merkle(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut layer: Vec<[u8; 32]> = leaves.to_vec();
    while layer.len() > 1 {
        let mut next = Vec::with_capacity((layer.len() + 1) / 2);
        for pair in layer.chunks(2) {
            let mut h = Sha256::new();
            h.update(pair[0]);
            h.update(if pair.len() == 2 { pair[1] } else { pair[0] });
            next.push(h.finalize().into());
        }
        layer = next;
    }
    layer[0]
}
