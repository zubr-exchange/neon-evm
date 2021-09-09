//! `EVMLoader` ERC20 Wrapper implementation

use std::convert::TryInto as _;

use solana_program::pubkey::Pubkey;

// ERC20 method ids:
//--------------------------------------------------
// totalSupply()                         => 18160ddd
// balanceOf(address)                    => 70a08231
// transfer(address,uint256)             => a9059cbb
// transferFrom(address,address,uint256) => 23b872dd
// approve(address,uint256)              => 095ea7b3
// allowance(address,address)            => dd62ed3e
//--------------------------------------------------

const ID_LEN: usize = 4;
const TOTAL_SUPPLY_ID: &[u8; ID_LEN] = &[0x18, 0x16, 0x0d, 0xdd];
const BALANCE_OF_ID: &[u8; ID_LEN] = &[0x70, 0xa0, 0x82, 0x31];
const TRANSFER_ID: &[u8; ID_LEN] = &[0xa9, 0x05, 0x9c, 0xbb];
const TRANSFER_FROM_ID: &[u8; ID_LEN] = &[0x23, 0xb8, 0x72, 0xdd];
const APPROVE_ID: &[u8; ID_LEN] = &[0x09, 0x5e, 0xa7, 0xb3];
const ALLOWANCE_ID: &[u8; ID_LEN] = &[0xdd, 0x62, 0xed, 0x3e];

/// Represents a ERC20 method.
pub enum Method {
    TotalSupply,
    BalanceOf,
    Transfer,
    TransferFrom,
    Approve,
    Allowance,
    Unknown,
}

/// Returns method by a 4-byte Ethereum method identifier.
pub fn method(id: &[u8]) -> Method {
    if id.len() != ID_LEN {
        return Method::Unknown;
    }
    let id: &[u8; ID_LEN] = id.try_into().expect("failed cast from slice into array");
    match id {
        TOTAL_SUPPLY_ID => Method::TotalSupply,
        BALANCE_OF_ID => Method::BalanceOf,
        TRANSFER_ID => Method::Transfer,
        TRANSFER_FROM_ID => Method::TransferFrom,
        APPROVE_ID => Method::Approve,
        ALLOWANCE_ID => Method::Allowance,
        _ => Method::Unknown,
    }
}

/// Returns total sum of all balances.
pub fn total_supply(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper totalSupply for token {})",
        token_mint
    );
    0
}

pub fn balance_of(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper balance_of for token {})",
        token_mint
    );
    0
}

pub fn transfer(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper transfer for token {})",
        token_mint
    );
    0
}

pub fn transfer_from(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper transfer_from for token {})",
        token_mint
    );
    0
}

pub fn approve(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper approve for token {})",
        token_mint
    );
    0
}

pub fn allowance(token_mint: Pubkey) -> u64 {
    debug_print!(
        "call_inner_erc20_wrapper allowance for token {})",
        token_mint
    );
    0
}
