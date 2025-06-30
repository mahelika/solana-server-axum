use axum::{
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
    Json, Router,
    response::Html
};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::{env, str::FromStr};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
struct GetBalance {
    wallet: String,
}

#[derive(Serialize)]
struct GetBalanceResponse {
    wallet: String,
    balance_lamports: u64,
    balance_sol: f64,
}

#[derive(Deserialize)]
struct AirdropRequest {
    wallet: String,
    sol: u64,
}

#[derive(Serialize)]
struct AirdropResponse {
    success: bool,
    message: String,
    wallet: String,
    airdrop_amount_sol: u64,
    transaction_signature: String,
    explorer_url: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    rpc_url: String,
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("../public/index.html"))
}

// async fn serve_html() -> impl axum::response::IntoResponse {
//     Html(include_str!("../static/index.html"))
// }

async fn health_check() -> ResponseJson<HealthResponse> {
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    ResponseJson(HealthResponse {
        status: "healthy".to_string(),
        rpc_url,
    })
}

async fn get_balance(
    Json(payload): Json<GetBalance>,
) -> Result<ResponseJson<GetBalanceResponse>, (StatusCode, ResponseJson<ErrorResponse>)> {
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let client = RpcClient::new(&rpc_url);
    
    let pubkey = match Pubkey::from_str(&payload.wallet) {
        Ok(key) => key,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                ResponseJson(ErrorResponse {
                    error: "Invalid wallet address".to_string(),
                }),
            ));
        }
    };

    let balance = match client.get_balance(&pubkey) {
        Ok(balance) => balance,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("Failed to get balance: {}", e),
                }),
            ));
        }
    };

    Ok(ResponseJson(GetBalanceResponse {
        wallet: payload.wallet,
        balance_lamports: balance,
        balance_sol: balance as f64 / LAMPORTS_PER_SOL as f64,
    }))
}

async fn get_airdrop(
    Json(payload): Json<AirdropRequest>,
) -> Result<ResponseJson<AirdropResponse>, (StatusCode, ResponseJson<ErrorResponse>)> {
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let client = RpcClient::new(&rpc_url);
    
    let pubkey = match Pubkey::from_str(&payload.wallet) {
        Ok(key) => key,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                ResponseJson(ErrorResponse {
                    error: "Invalid wallet address".to_string(),
                }),
            ));
        }
    };

    let lamports_amount = payload.sol * LAMPORTS_PER_SOL;
    
    if lamports_amount > 2 * LAMPORTS_PER_SOL {
        return Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(ErrorResponse {
                error: "Airdrop amount too large (max 2 SOL)".to_string(),
            }),
        ));
    }

    let sig = match client.request_airdrop(&pubkey, lamports_amount) {
        Ok(sig) => sig,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("Airdrop failed: {}", e),
                }),
            ));
        }
    };

    let explorer_url = format!("https://explorer.solana.com/tx/{}?cluster=devnet", sig);
    
    println!("Airdrop txn: {}", explorer_url);

    Ok(ResponseJson(AirdropResponse {
        success: true,
        message: format!("Airdrop of {} SOL requested successfully! Use the 'Check Balance' button to see your updated balance.", payload.sol),
        wallet: payload.wallet,
        airdrop_amount_sol: payload.sol,
        transaction_signature: sig.to_string(),
        explorer_url,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("RPC_URL").is_err() {
        env::set_var("RPC_URL", "https://api.devnet.solana.com");
    }

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/health", get(health_check))
        .route("/get_balance", post(get_balance))
        .route("/get_airdrop", post(get_airdrop))
        .layer(CorsLayer::permissive()); 

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let listener = TcpListener::bind(&addr).await?;
    println!("listening on http://localhost:{}", port);
    println!("health check: http://localhost:{}/health", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}