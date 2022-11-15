use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Escrow expired ")]
    Expired {},

    #[error("Buying Utility Over Slippages")]
    BuyingUtilityOverSlippages {},
    
    #[error("FEE_MORE_THAN_AMOUNT")]
    InsufficientToken {},

    #[error("FEE_MORE_THAN_AMOUNT")]
    InsufficientEthToSwap {},

    #[error("PoolAndTokenMismatch")]
    PoolAndTokenMismatch {},

    #[error("NativeInputZero")]
    NativeInputZero {},

    #[error("TokenTypeMismatch")]
    TokenTypeMismatch {},

    #[error("Cw20InputZero")]
    Cw20InputZero {},

    #[error("Expired")]
    NotExpired {},
}
