use cosmwasm_std::{
    entry_point, Addr, DepsMut, Env, MessageInfo, Response,
    Uint128, Uint64, CosmosMsg
};

use cw20::Denom;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, State, BOT_ROLES};
use crate::util;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
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
        ExecuteMsg::BuyToken {juno_amount, token, token_amount_per_native, slippage_bips, to, router, platform_fee_bips, gas_estimate, deadline} => 
                buy_token(deps, &mut state, info, env, juno_amount, token, token_amount_per_native, slippage_bips, to, router, platform_fee_bips, gas_estimate, deadline)
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

    state.owner = new_admin;
    config(deps.storage).save(&state)?;

    Ok(Response::new()
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
    token: Addr,
    _token_amount_per_native: Uint128,
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

    let mut _juno_amount = juno_amount;
    _juno_amount -= gas_estimate;

    let platform_fee = platform_fee_bips * juno_amount / Uint128::from(10000u128);
    state.pending_platform_fee += platform_fee;
    //let approxTxFee = gas_estimate * tx.gasprice;
    _juno_amount -= platform_fee;

    if juno_amount <= Uint128::zero() {
        return Err(ContractError::InsufficientEthToSwap{});
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    let (token2_amount, token2_denom, mut messages_swap) = 
        util::get_swap_amount_and_denom_and_message(deps.querier
            , pool
            , Denom::Native(String::from("ujuno"))
            , juno_amount
            , recipient)?;
    messages.append(&mut messages_swap);    

    Ok(Response::new()
        .add_messages(messages))
}

// #[entry_point]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//         QueryMsg::Arbiter {} => to_binary(&query_arbiter(deps)?),
//     }
// }

// fn query_arbiter(deps: Deps) -> StdResult<ArbiterResponse> {
//     let state = config_read(deps.storage).load()?;
//     let addr = state.arbiter;
//     Ok(ArbiterResponse { arbiter: addr })
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, CosmosMsg, Timestamp};

//     fn init_msg_expire_by_height(height: u64) -> InstantiateMsg {
//         InstantiateMsg {
//             arbiter: String::from("verifies"),
//             recipient: String::from("benefits"),
//             end_height: Some(height),
//             end_time: None,
//         }
//     }

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(&[]);

//         let msg = init_msg_expire_by_height(1000);
//         let mut env = mock_env();
//         env.block.height = 876;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("creator", &coins(1000, "earth"));

//         let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let state = config_read(&mut deps.storage).load().unwrap();
//         assert_eq!(
//             state,
//             State {
//                 arbiter: Addr::unchecked("verifies"),
//                 recipient: Addr::unchecked("benefits"),
//                 source: Addr::unchecked("creator"),
//                 end_height: Some(1000),
//                 end_time: None,
//             }
//         );
//     }

//     #[test]
//     fn cannot_initialize_expired() {
//         let mut deps = mock_dependencies(&[]);

//         let msg = init_msg_expire_by_height(1000);
//         let mut env = mock_env();
//         env.block.height = 1001;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("creator", &coins(1000, "earth"));

//         let res = instantiate(deps.as_mut(), env, info, msg);
//         match res.unwrap_err() {
//             ContractError::Expired { .. } => {}
//             e => panic!("unexpected error: {:?}", e),
//         }
//     }

//     #[test]
//     fn init_and_query() {
//         let mut deps = mock_dependencies(&[]);

//         let arbiter = Addr::unchecked("arbiters");
//         let recipient = Addr::unchecked("receives");
//         let creator = Addr::unchecked("creates");
//         let msg = InstantiateMsg {
//             arbiter: arbiter.clone().into(),
//             recipient: recipient.into(),
//             end_height: None,
//             end_time: None,
//         };
//         let mut env = mock_env();
//         env.block.height = 876;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info(creator.as_str(), &[]);
//         let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // now let's query
//         let query_response = query_arbiter(deps.as_ref()).unwrap();
//         assert_eq!(query_response.arbiter, arbiter);
//     }

//     #[test]
//     fn execute_approve() {
//         let mut deps = mock_dependencies(&[]);

//         // initialize the store
//         let init_amount = coins(1000, "earth");
//         let msg = init_msg_expire_by_height(1000);
//         let mut env = mock_env();
//         env.block.height = 876;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("creator", &init_amount);
//         let contract_addr = env.clone().contract.address;
//         let init_res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(0, init_res.messages.len());

//         // balance changed in init
//         deps.querier.update_balance(&contract_addr, init_amount);

//         // beneficiary cannot release it
//         let msg = ExecuteMsg::Approve { quantity: None };
//         let mut env = mock_env();
//         env.block.height = 900;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("beneficiary", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone());
//         match execute_res.unwrap_err() {
//             ContractError::Unauthorized { .. } => {}
//             e => panic!("unexpected error: {:?}", e),
//         }

//         // verifier cannot release it when expired
//         let mut env = mock_env();
//         env.block.height = 1100;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("verifies", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone());
//         match execute_res.unwrap_err() {
//             ContractError::Expired { .. } => {}
//             e => panic!("unexpected error: {:?}", e),
//         }

//         // complete release by verfier, before expiration
//         let mut env = mock_env();
//         env.block.height = 999;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("verifies", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone()).unwrap();
//         assert_eq!(1, execute_res.messages.len());
//         let msg = execute_res.messages.get(0).expect("no message");
//         assert_eq!(
//             msg.msg,
//             CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "benefits".into(),
//                 amount: coins(1000, "earth"),
//             })
//         );

//         // partial release by verfier, before expiration
//         let partial_msg = ExecuteMsg::Approve {
//             quantity: Some(coins(500, "earth")),
//         };
//         let mut env = mock_env();
//         env.block.height = 999;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("verifies", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, partial_msg).unwrap();
//         assert_eq!(1, execute_res.messages.len());
//         let msg = execute_res.messages.get(0).expect("no message");
//         assert_eq!(
//             msg.msg,
//             CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "benefits".into(),
//                 amount: coins(500, "earth"),
//             })
//         );
//     }

//     #[test]
//     fn handle_refund() {
//         let mut deps = mock_dependencies(&[]);

//         // initialize the store
//         let init_amount = coins(1000, "earth");
//         let msg = init_msg_expire_by_height(1000);
//         let mut env = mock_env();
//         env.block.height = 876;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("creator", &init_amount);
//         let contract_addr = env.clone().contract.address;
//         let init_res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(0, init_res.messages.len());

//         // balance changed in init
//         deps.querier.update_balance(&contract_addr, init_amount);

//         // cannot release when unexpired (height < end_height)
//         let msg = ExecuteMsg::Refund {};
//         let mut env = mock_env();
//         env.block.height = 800;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("anybody", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone());
//         match execute_res.unwrap_err() {
//             ContractError::NotExpired { .. } => {}
//             e => panic!("unexpected error: {:?}", e),
//         }

//         // cannot release when unexpired (height == end_height)
//         let msg = ExecuteMsg::Refund {};
//         let mut env = mock_env();
//         env.block.height = 1000;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("anybody", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone());
//         match execute_res.unwrap_err() {
//             ContractError::NotExpired { .. } => {}
//             e => panic!("unexpected error: {:?}", e),
//         }

//         // anyone can release after expiration
//         let mut env = mock_env();
//         env.block.height = 1001;
//         env.block.time = Timestamp::from_seconds(0);
//         let info = mock_info("anybody", &[]);
//         let execute_res = execute(deps.as_mut(), env, info, msg.clone()).unwrap();
//         assert_eq!(1, execute_res.messages.len());
//         let msg = execute_res.messages.get(0).expect("no message");
//         assert_eq!(
//             msg.msg,
//             CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "creator".into(),
//                 amount: coins(1000, "earth"),
//             })
//         );
//     }
// }
