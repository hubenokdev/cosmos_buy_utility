use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    Uint128, Uint64, CosmosMsg,
    StdResult,
};

use cw20::Denom;

use crate::error::ContractError;
use crate::msg::{AdminResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, State, BOT_ROLES};
use crate::util;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
        pending_platform_fee: Uint128::zero(),
    };

    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;
    match msg {
        ExecuteMsg::WithdrawFee { to, amount } => try_withdraw_fee(deps, &mut state, info, to, amount),
        ExecuteMsg::SetAdmin { new_admin } => try_set_admin(deps, &mut state, info, new_admin),
        ExecuteMsg::SetBotRole { new_bot, enabled } => try_set_bot_role(deps, state, info, new_bot, enabled),
        ExecuteMsg::BuyToken {juno_amount, token_amount_per_native, slippage_bips, to, pool_address, platform_fee_bips, gas_estimate, deadline} => 
                buy_token(deps, &mut state, info, env, juno_amount, token_amount_per_native, slippage_bips, to, pool_address, platform_fee_bips, gas_estimate, deadline),
    }
}

fn try_set_admin(
    deps: DepsMut,
    state: &mut State,
    info: MessageInfo,
    new_admin: Addr
) -> Result<Response, ContractError> {
    if state.owner != info.sender {
        return Err(ContractError::Unauthorized { });
    }

    state.owner = new_admin.clone();
    config(deps.storage).save(&state)?;

    Ok(Response::new()
        .add_attribute("new_admin", new_admin)
    )
}

fn try_set_bot_role(
    deps: DepsMut,
    state: State,
    info: MessageInfo,
    new_bot: Addr,
    role: bool
) -> Result<Response, ContractError> {
    if state.owner != info.sender {
        return Err(ContractError::Unauthorized { });
    }

    BOT_ROLES.save(deps.storage, new_bot, &role)?;

    Ok(Response::new()
    )
}

fn try_withdraw_fee(
    deps: DepsMut,
    state: &mut State,
    info: MessageInfo,
    to: Addr,
    amount: Uint128
) -> Result<Response, ContractError> {
    if state.owner != info.sender {
        return Err(ContractError::Unauthorized { });
    }

    state.pending_platform_fee -= amount;

    config(deps.storage).save(&state)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    msgs.push(util::transfer_token_message(Denom::Native(String::from("ujuno")), amount, to)?);

    Ok(Response::new()
        .add_messages(msgs)
    )
}

fn buy_token(
    deps: DepsMut,
    state: &mut State,
    info: MessageInfo,
    env: Env,
    juno_amount: Uint128,
    //token: Addr,
    token_amount_per_native: Uint128,
    slippage_bips: Uint128,
    recipient: Addr,
    pool: Addr,
    platform_fee_bips: Uint128,
    gas_estimate: Uint128,
    deadline: Uint64,
) -> Result<Response, ContractError> {

    if !BOT_ROLES.has(deps.storage, info.sender.clone()) {
        return Err(ContractError::Unauthorized {});    
    }
    let enabled = BOT_ROLES.load(deps.storage, info.sender)?;
    if !enabled {
        return Err(ContractError::Unauthorized {});    
    }

    if env.block.time.nanos() <= deadline.u64() {
        return Err(ContractError::Expired { });
    }

    if slippage_bips <= Uint128::from(10000u128) {
        return Err(ContractError::BuyingUtilityOverSlippages { });
    }

    if gas_estimate <= juno_amount {
        return Err(ContractError::InsufficientToken{});
    }

    let mut _juno_amount = juno_amount - gas_estimate;

    let platform_fee = platform_fee_bips * juno_amount / Uint128::from(10000u128);
    state.pending_platform_fee += platform_fee;
    //let approxTxFee = gas_estimate * tx.gasprice;
    let amount_out_min = _juno_amount * token_amount_per_native * (Uint128::from(10000u128) - slippage_bips) / Uint128::from(10000000000u128);
    _juno_amount -= platform_fee;

    if juno_amount <= Uint128::zero() {
        return Err(ContractError::InsufficientEthToSwap{});
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    let (_token2_amount, _token2_denom, mut messages_swap) = 
        util::get_swap_amount_and_denom_and_message(deps.querier
            , pool
            , Denom::Native(String::from("ujuno"))
            , juno_amount
            , amount_out_min
            , recipient)?;
    messages.append(&mut messages_swap);    

    Ok(Response::new()
        .add_messages(messages))
}

// fn try_refund(
//     deps: DepsMut,
//     env: Env,
//     _info: MessageInfo,
//     recipient: Addr,
//     _refund_amount: Uint128
// ) -> Result<Response, ContractError> {
//     // anyone can try to refund, as long as the contract is expired
//     // if !state.is_expired(&env) {
//     //     return Err(ContractError::NotExpired {});
//     // }

//     // Querier guarantees to returns up-to-date data, including funds sent in this handle message
//     // https://github.com/CosmWasm/wasmd/blob/master/x/wasm/internal/keeper/keeper.go#L185-L192
//     let balance = deps.querier.query_all_balances(&env.contract.address)?;
//     Ok(send_tokens(recipient, balance, "refund"))
// }

// // this is a helper to move the tokens, so the business logic is easy to read
// fn send_tokens(to_address: Addr, amount: Vec<Coin>, action: &str) -> Response {
//     Response::new()
//         .add_message(BankMsg::Send {
//             to_address: to_address.clone().into(),
//             amount,
//         })
//         .add_attribute("action", action)
//         .add_attribute("to", to_address)
// }

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAdmin {} => to_binary(&query_admin(deps)?),
        // QueryMsg::GetBots {} => to_binary(&query_admin(deps)?),
    }
}

fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let state = config_read(deps.storage).load()?;
    let admin = state.owner;
    Ok(AdminResponse { admin })
}

