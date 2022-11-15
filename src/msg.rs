use cosmwasm_std::{Addr, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub arbiter: String,
    pub recipient: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    WithdrawFee {
        // release some coins - if quantity is None, release all coins in balance
        to: Addr,
        amount: Uint128,
    },
    SetAdmin {
        new_admin: Addr,
    },
    SetBotRole {
        new_bot: Addr,
        enabled: bool
    },
    BuyToken { 
        juno_amount: Uint128,
        token: Addr,
        token_amount_per_native: Uint128,
        slippage_bips: Uint128,
        to: Addr,
        router: Addr,
        platform_fee_bips: Uint128,
        gas_estimate: Uint128,
        deadline: Uint64
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns a human-readable representation of the arbiter.
    Arbiter {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ArbiterResponse {
    pub arbiter: Addr,
}
