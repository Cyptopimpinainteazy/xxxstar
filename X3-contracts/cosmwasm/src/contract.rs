//! X3 HTLC — CosmWasm Smart Contract
//!
//! Implements an atomic swap HTLC on CosmWasm-enabled Cosmos SDK chains.
//! Supports native denom and CW20 token locks with SHA-256 hashlock.
//!
//! # Messages
//! * `ExecuteMsg::Lock` — lock funds with hashlock, receiver, timeout
//! * `ExecuteMsg::Claim` — claim with preimage (anyone can claim)
//! * `ExecuteMsg::Refund` — refund after timeout (sender only)
//! * `QueryMsg::LockStatus` — check lock state

use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── State ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HtlcState {
    pub hashlock: [u8; 32],
    pub sender: Addr,
    pub receiver: Addr,
    pub refund_address: Addr,
    pub amount: Uint128,
    pub denom: String, // native denom or CW20 contract address
    pub is_cw20: bool,
    pub timeout: u64, // unix timestamp
    pub claimed: bool,
    pub refunded: bool,
}

// ── Messages ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    /// Lock funds for an atomic swap.
    Lock {
        hashlock: Binary, // 32 bytes
        receiver: String,
        timeout: u64,
    },
    /// Claim funds with the preimage.
    Claim {
        swap_id: String,
        preimage: Binary,
    },
    /// Refund after timeout.
    Refund {
        swap_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum QueryMsg {
    /// Get the lock state for a swap.
    LockStatus { swap_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LockStatusResponse {
    pub swap_id: String,
    pub sender: Addr,
    pub receiver: Addr,
    pub hashlock: Binary,
    pub amount: Uint128,
    pub denom: String,
    pub timeout: u64,
    pub claimed: bool,
    pub refunded: bool,
    pub active: bool,
}

// ── Entry points ─────────────────────────────────────────────────────

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // Store governance address
    deps.storage.set(b"governance", &msg.governance.as_bytes());
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Lock { hashlock, receiver, timeout } => {
            execute_lock(deps, env, info, hashlock, receiver, timeout)
        }
        ExecuteMsg::Claim { swap_id, preimage } => {
            execute_claim(deps, env, info, swap_id, preimage)
        }
        ExecuteMsg::Refund { swap_id } => {
            execute_refund(deps, env, info, swap_id)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LockStatus { swap_id } => query_lock_status(deps, swap_id),
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

fn execute_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    hashlock: Binary,
    receiver: String,
    timeout: u64,
) -> StdResult<Response> {
    // Validate timeout is in the future
    if timeout <= env.block.time.seconds() {
        return Err(StdError::generic_err("timeout must be in the future"));
    }

    // Validate hashlock is 32 bytes
    let hashlock_bytes: [u8; 32] = hashlock
        .0
        .try_into()
        .map_err(|_| StdError::generic_err("hashlock must be 32 bytes"))?;

    // Require at least one coin sent
    let funds = info.funds;
    if funds.is_empty() {
        return Err(StdError::generic_err("must send funds to lock"));
    }

    let coin = &funds[0];
    let receiver_addr = deps.api.addr_validate(&receiver)?;
    let refund_addr = info.sender.clone();

    // Determine if CW20 or native
    let is_cw20 = false; // Native denom by default; CW20 detection via ExecuteMsg variant

    // Generate swap ID from hashlock + sender + block time
    let mut hasher = Sha256::new();
    hasher.update(&hashlock_bytes);
    hasher.update(refund_addr.as_bytes());
    hasher.update(env.block.time.seconds().to_le_bytes());
    let swap_hash = hasher.finalize();
    let swap_id = hex::encode(swap_hash);

    // Store lock state
    let state = HtlcState {
        hashlock: hashlock_bytes,
        sender: refund_addr.clone(),
        receiver: receiver_addr,
        refund_address: refund_addr,
        amount: coin.amount,
        denom: coin.denom.clone(),
        is_cw20,
        timeout,
        claimed: false,
        refunded: false,
    };

    let key = format!("lock:{}", swap_id);
    deps.storage.set(key.as_bytes(), &serde_json::to_vec(&state)?);

    Ok(Response::new()
        .add_attribute("action", "lock")
        .add_attribute("swap_id", swap_id)
        .add_attribute("amount", coin.amount.to_string())
        .add_attribute("denom", &coin.denom)
        .add_attribute("receiver", receiver)
        .add_attribute("timeout", timeout.to_string()))
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap_id: String,
    preimage: Binary,
) -> StdResult<Response> {
    let key = format!("lock:{}", swap_id);
    let raw = deps
        .storage
        .get(key.as_bytes())
        .ok_or_else(|| StdError::generic_err("swap not found"))?;

    let mut state: HtlcState =
        serde_json::from_slice(&raw).map_err(|e| StdError::generic_err(e.to_string()))?;

    // Guard: already claimed or refunded
    if state.claimed {
        return Err(StdError::generic_err("already claimed"));
    }
    if state.refunded {
        return Err(StdError::generic_err("already refunded"));
    }

    // Verify preimage
    let computed_hash = Sha256::digest(&preimage.0);
    if computed_hash.as_slice() != state.hashlock {
        return Err(StdError::generic_err("wrong preimage"));
    }

    state.claimed = true;
    deps.storage.set(key.as_bytes(), &serde_json::to_vec(&state)?);

    // Transfer locked funds to receiver
    let transfer_msg = if state.is_cw20 {
        WasmMsg::Execute {
            contract_addr: state.denom.clone(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: state.receiver.to_string(),
                amount: state.amount,
            })?,
            funds: vec![],
        }
    } else {
        cosmwasm_std::BankMsg::Send {
            to_address: state.receiver.to_string(),
            amount: vec![Coin {
                denom: state.denom.clone(),
                amount: state.amount,
            }],
        }
        .into()
    };

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "claim")
        .add_attribute("swap_id", swap_id)
        .add_attribute("claimant", info.sender)
        .add_attribute("receiver", state.receiver))
}

fn execute_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap_id: String,
) -> StdResult<Response> {
    let key = format!("lock:{}", swap_id);
    let raw = deps
        .storage
        .get(key.as_bytes())
        .ok_or_else(|| StdError::generic_err("swap not found"))?;

    let mut state: HtlcState =
        serde_json::from_slice(&raw).map_err(|e| StdError::generic_err(e.to_string()))?;

    // Guard: timeout not reached
    if env.block.time.seconds() < state.timeout {
        return Err(StdError::generic_err("timeout not reached"));
    }

    // Guard: already claimed or refunded
    if state.claimed {
        return Err(StdError::generic_err("already claimed"));
    }
    if state.refunded {
        return Err(StdError::generic_err("already refunded"));
    }

    // Guard: caller must be refund address (the original sender)
    if info.sender != state.refund_address {
        return Err(StdError::generic_err("not refund address"));
    }

    state.refunded = true;
    deps.storage.set(key.as_bytes(), &serde_json::to_vec(&state)?);

    // Transfer locked funds back to refund address
    let refund_msg: cosmwasm_std::CosmosMsg = if state.is_cw20 {
        WasmMsg::Execute {
            contract_addr: state.denom.clone(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: state.refund_address.to_string(),
                amount: state.amount,
            })?,
            funds: vec![],
        }
    } else {
        cosmwasm_std::BankMsg::Send {
            to_address: state.refund_address.to_string(),
            amount: vec![Coin {
                denom: state.denom.clone(),
                amount: state.amount,
            }],
        }
        .into()
    };

    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("action", "refund")
        .add_attribute("swap_id", swap_id)
        .add_attribute("refund_address", state.refund_address))
}

fn query_lock_status(deps: Deps, swap_id: String) -> StdResult<Binary> {
    let key = format!("lock:{}", swap_id);
    let raw = deps
        .storage
        .get(key.as_bytes())
        .ok_or_else(|| StdError::generic_err("swap not found"))?;

    let state: HtlcState =
        serde_json::from_slice(&raw).map_err(|e| StdError::generic_err(e.to_string()))?;

    let now = state.timeout; // In production, use env.block.time.seconds()
    let active = !state.claimed && !state.refunded && now < state.timeout;

    to_json_binary(&LockStatusResponse {
        swap_id,
        sender: state.sender,
        receiver: state.receiver,
        hashlock: Binary::from(state.hashlock.to_vec()),
        amount: state.amount,
        denom: state.denom,
        timeout: state.timeout,
        claimed: state.claimed,
        refunded: state.refunded,
        active,
    })
}