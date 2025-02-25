use evm::backend::Apply;
use evm::{H160, U256};
#[allow(unused)]
use solana_sdk::{
    pubkey::Pubkey,
    account::Account,
    commitment_config::CommitmentConfig,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::invoke_signed,
    transaction::Transaction,
    signer::keypair::Keypair,
    signature::Signature,
    signer::Signer,
    program_error::ProgramError,
    transaction::TransactionError,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::process::exit;
use evm_loader::{
    account_data::{AccountData, ACCOUNT_SEED_VERSION, Contract},
    hamt::Hamt,
    solana_backend::AccountStorage,
    solidity_account::SolidityAccount,
    precompile_contracts::is_precompile_address,
    executor_state::{SplTransfer, SplApprove, ERC20Approve}
};
#[allow(unused)]
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    rc::Rc,
    error,
    time::Duration,
    thread::sleep,
    convert::TryFrom,
};
use crate::Config;
#[allow(unused)]
use solana_program::{
    instruction::Instruction,
    instruction::AccountMeta,
    message::Message,
    native_token::lamports_to_sol,
};
#[allow(unused)]
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::RpcSimulateTransactionConfig,
    client_error,
    client_error::reqwest::StatusCode,
    rpc_request::MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS,
};


#[derive(Debug, Clone)]
pub struct TokenAccount {
    owner: Pubkey,
    contract: Pubkey,
    mint: Pubkey,
    key: Pubkey,
    new: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenAccountJSON {
    owner: String,
    contract: String,
    mint: String,
    key: String,
    new: bool
}
impl From<TokenAccount> for TokenAccountJSON {
    fn from(account: TokenAccount) -> Self {
        Self {
            owner: bs58::encode(&account.owner).into_string(),
            contract: bs58::encode(&account.contract).into_string(),
            mint: bs58::encode(&account.mint).into_string(),
            key: bs58::encode(&account.key).into_string(),
            new: account.new,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountJSON {
    address: String,
    account: String,
    contract: Option<String>,
    writable: bool,
    new: bool,
    code_size: Option<usize>,
    code_size_current: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolanaAccountJSON {
    /// An account's public key
    pub pubkey: String,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}
impl From<AccountMeta> for SolanaAccountJSON {
    fn from(account_meta: AccountMeta) -> Self {
        Self {
            pubkey: bs58::encode(&account_meta.pubkey).into_string(),
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

struct SolanaAccount {
    account: Account,
    code_account: Option<Account>,
    key: Pubkey,
    writable: bool,
    code_size: Option<usize>,
    code_size_current: Option<usize>,
    balance: u64,
}

struct SolanaNewAccount {
    key: Pubkey,
    writable: bool,
    code_size: Option<usize>
}

impl SolanaAccount {
    pub fn new(account: Account, key: Pubkey, balance: u64, code_account: Option<Account>) -> Self {
        eprintln!("SolanaAccount::new");
        Self{account, key, balance, writable: false, code_account, code_size: None, code_size_current : None}
    }
}

impl SolanaNewAccount {
    pub const fn new(key: Pubkey) -> Self {
        Self{key, writable: false, code_size: None}
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct EmulatorAccountStorage<'a> {
    accounts: RefCell<HashMap<H160, SolanaAccount>>,
    new_accounts: RefCell<HashMap<H160, SolanaNewAccount>>,
    pub solana_accounts: RefCell<HashMap<Pubkey, AccountMeta>>,
    pub token_accounts: RefCell<HashMap<Pubkey, TokenAccount>>,
    config: &'a Config,
    contract_id: H160,
    caller_id: H160,
    block_number: u64,
    block_timestamp: i64,
}

impl<'a> EmulatorAccountStorage<'a> {
    pub fn new(config: &'a Config, contract_id: H160, caller_id: H160) -> EmulatorAccountStorage {
        eprintln!("backend::new");

        let slot = if let Ok(slot) = config.rpc_client.get_slot() {
            eprintln!("Got slot");
            eprintln!("Slot {}", slot);
            slot
        }
        else {
            eprintln!("Get slot error");
            0
        };

        let timestamp = if let Ok(timestamp) = config.rpc_client.get_block_time(slot) {
            eprintln!("Got timestamp");
            eprintln!("timestamp {}", timestamp);
            timestamp
        } else {
            eprintln!("Get timestamp error");
            0
        };

        Self {
            accounts: RefCell::new(HashMap::new()),
            new_accounts: RefCell::new(HashMap::new()),
            solana_accounts: RefCell::new(HashMap::new()),
            token_accounts: RefCell::new(HashMap::new()),
            config,
            contract_id,
            caller_id,
            block_number: slot,
            block_timestamp: timestamp,
        }
    }

    pub fn get_account_from_solana(config: &'a Config, address: &H160) -> Option<(Account, u64, Option<Account>)> {
        let (solana_address, _solana_nonce) = make_solana_program_address(address, &config.evm_loader);
        eprintln!("Not found account for 0x{} => {}", &hex::encode(&address.as_fixed_bytes()), &solana_address.to_string());

        if let Some(acc) = config.rpc_client.get_account_with_commitment(&solana_address, CommitmentConfig::processed()).unwrap().value {
            eprintln!("Account found");
            eprintln!("Account data len {}", acc.data.len());
            eprintln!("Account owner {}", acc.owner.to_string());

            let account_data = match AccountData::unpack(&acc.data) {
                Ok(acc_data) => match acc_data {
                    AccountData::Account(acc) => acc,
                    _ => return None,
                },
                Err(_) => return None,
            };

            let code_account = if account_data.code_account == Pubkey::new_from_array([0_u8; 32]) {
                eprintln!("code_account == Pubkey::new_from_array([0u8; 32])");
                None
            } else {
                eprintln!("code_account != Pubkey::new_from_array([0u8; 32])");
                eprintln!("account key:  {}", &solana_address.to_string());
                eprintln!("code account: {}", &account_data.code_account.to_string());

                if let Some(acc) = config.rpc_client.get_account_with_commitment(&account_data.code_account, CommitmentConfig::processed()).unwrap().value {
                    eprintln!("Account found");
                    Some(acc)
                }
                else {
                    eprintln!("Account not found");
                    None
                }
            };
            let token_amount = config.rpc_client.get_token_account_balance_with_commitment(&account_data.eth_token_account, CommitmentConfig::processed()).unwrap().value;
            let balance = token_amount.amount.parse::<u64>().unwrap();

            Some((acc, balance, code_account))
        }
        else {
            eprintln!("Account not found {}", &address.to_string());

            None
        }
    }

    fn create_acc_if_not_exists(&self, address: &H160) -> bool {
        let mut accounts = self.accounts.borrow_mut(); 
        let mut new_accounts = self.new_accounts.borrow_mut(); 
        if accounts.get(address).is_none() {
            let (solana_address, _solana_nonce) = make_solana_program_address(address, &self.config.evm_loader);
            if let Some((acc, balance, code_account)) = Self::get_account_from_solana(self.config, address) {
                accounts.insert(*address, SolanaAccount::new(acc, solana_address, balance, code_account));
                true
            }
            else {
                eprintln!("Account not found {}", &address.to_string());
                new_accounts.insert(*address, SolanaNewAccount::new(solana_address));
                false
            }
        } else {
            true
        }
    }

    // pub fn make_solidity_account<'a>(self, account:&'a SolanaAccount) -> SolidityAccount<'a> {
    //     let mut data = account.account.data.clone();
    //     let data_rc: std::rc::Rc<std::cell::RefCell<&mut [u8]>> = Rc::new(RefCell::new(&mut data));
    //     SolidityAccount::new(&account.key, data_rc, account.account.lamports).unwrap()
    // }


    pub fn apply<A, I>(&self, values: A)
            where
                A: IntoIterator<Item=Apply<I>>,
                I: IntoIterator<Item=(U256, U256)>,
    {

        let mut accounts = self.accounts.borrow_mut();
        let mut new_accounts = self.new_accounts.borrow_mut();

        for apply in values {
            match apply {
                Apply::Modify {address, nonce, code_and_valids, storage, reset_storage} => {

                    let code_begin;
                    let code_size;
                    let valids_size;

                    let mut storage_iter = storage.into_iter().peekable();
                    let exist_items: bool = matches!(storage_iter.peek(), Some(_));

                    let hamt_size = |code_data : &Vec<u8>, hamt_begin : usize| -> usize {
                        let mut empty_data: Vec<u8> = Vec::new();
                        empty_data.resize(10_485_760, 0);
                        empty_data[0..code_data.len()].copy_from_slice(code_data);

                        let mut storage = Hamt::new(&mut empty_data[hamt_begin..], reset_storage).unwrap();
                        for (key, value) in storage_iter {
                            eprintln!("Storage value: {} = {}", &key.to_string(), &value.to_string());
                            storage.insert(key, value).unwrap();
                        }
                        storage.last_used() as usize
                    };


                    if let Some(acc) = accounts.get_mut(&address) {

                        let account_data = AccountData::unpack(&acc.account.data).unwrap();
                        if let AccountData::Account(acc_desc) = account_data {
                            if let Some(ref mut code_account) = acc.code_account{

                                let account_data_contract = AccountData::unpack(&code_account.data).unwrap();
                                let contract = AccountData::get_contract(&account_data_contract).unwrap();

                                if let Some((code, valids)) = code_and_valids.clone() {
                                    if contract.code_size != 0 {
                                        eprintln!("AccountAlreadyInitialized; account={:?}, code_account={:?}", acc.key, acc_desc.code_account );
                                        exit(1)
                                    }
                                    code_begin = AccountData::Contract( Contract {owner: Pubkey::new_from_array([0_u8; 32]), code_size: 0_u32} ).size();
                                    code_size = code.len();
                                    valids_size = valids.len();
                                }
                                else{
                                    if contract.code_size == 0 {
                                        eprintln!("UninitializedAccount; account={:?}, code_account={:?}", acc.key, acc_desc.code_account );
                                        exit(1)
                                    }
                                    code_begin = account_data_contract.size();
                                    code_size = contract.code_size as usize;
                                    valids_size = (code_size / 8) + 1;
                                }

                                let hamt_begin = code_begin + code_size + valids_size;

                                *acc.code_size.borrow_mut() = Some(hamt_begin + hamt_size(&code_account.data, hamt_begin));
                                *acc.code_size_current.borrow_mut() = Some(code_account.data.len());

                                let trx_count = u64::try_from(nonce).map_err(|s| {eprintln!("convert nonce error, {:?}", s); exit(1)}).unwrap();

                                if reset_storage || exist_items || code_and_valids.is_some() || acc_desc.trx_count != trx_count {
                                    *acc.writable.borrow_mut() = true;
                                }
                            }
                            else{
                                if reset_storage || exist_items || code_and_valids.is_some() {
                                    eprintln!("changes to the storage can only be applied to the contract account; existing address: {}", &address.to_string());
                                    exit(1);
                                }
                                *acc.writable.borrow_mut() = true;
                            }

                        }
                        else{
                            eprintln!("Changes of incorrect account were found {}", &address.to_string());
                            exit(1);
                        }
                    }
                    else if let Some(acc) = new_accounts.get_mut(&address) {
                        if let Some((code, valids)) = code_and_valids.clone() {
                            code_begin = AccountData::Contract( Contract {owner: Pubkey::new_from_array([0_u8; 32]), code_size: 0_u32} ).size();
                            code_size = code.len();
                            valids_size = valids.len();

                            let hamt_begin = code_begin + code_size + valids_size;
                            *acc.code_size.borrow_mut() = Some(hamt_begin + hamt_size(&vec![0_u8; 0], hamt_begin));
                        }
                        else  if reset_storage || exist_items {
                                eprintln!("changes to the storage can only be applied to the contract account; new address: {}", &address.to_string());
                                exit(1);
                            }

                        *acc.writable.borrow_mut() = true;
                    }
                    else {
                        eprintln!("Account not found {}", &address.to_string());
                    }
                    eprintln!("Modify: {} {} {}", &address.to_string(), &nonce.as_u64(), &reset_storage.to_string());
                },
                Apply::Delete {address: addr} => {
                    eprintln!("Delete: {}", addr.to_string());
                },
            }
        };
    }

    pub fn apply_spl_transfers(&self, transfers: Vec<SplTransfer>) {
        let mut token_accounts = self.token_accounts.borrow_mut();
        for transfer in transfers {
            self.create_acc_if_not_exists(&transfer.source);
            self.create_acc_if_not_exists(&transfer.target);

            let (contract_solana_address, _) = make_solana_program_address(&transfer.contract, &self.config.evm_loader);

            let (source_solana_address, _) = make_solana_program_address(&transfer.source, &self.config.evm_loader);
            token_accounts.entry(transfer.source_token).or_insert(
                TokenAccount {
                    owner: source_solana_address,
                    contract: contract_solana_address,
                    mint: transfer.mint,
                    key: transfer.source_token,
                    new: false
                }
            );

            let ui_token_account = self.config.rpc_client.get_token_account_with_commitment(&transfer.target_token, CommitmentConfig::processed());
            let target_token_exists = ui_token_account.map(|r| r.value.is_some()).unwrap_or(false);

            let (target_solana_address, _) = make_solana_program_address(&transfer.target, &self.config.evm_loader);
            token_accounts.entry(transfer.target_token).or_insert(
                TokenAccount {
                    owner: target_solana_address,
                    contract: contract_solana_address,
                    mint: transfer.mint,
                    key: transfer.target_token,
                    new: !target_token_exists
                }
            );
        } 
    }

    pub fn apply_spl_approves(&self, approves: Vec<SplApprove>) {
        let mut token_accounts = self.token_accounts.borrow_mut();

        for approve in approves {
            self.create_acc_if_not_exists(&approve.owner);

            let (contract_solana_address, _) = make_solana_program_address(&approve.contract, &self.config.evm_loader);
            let (owner_solana_address, _) = make_solana_program_address(&approve.owner, &self.config.evm_loader);

            let (token_address, _) = self.get_erc20_token_address(&approve.owner, &approve.contract, &approve.mint);
            let ui_token_account = self.config.rpc_client.get_token_account_with_commitment(&token_address, CommitmentConfig::processed());
            let token_exists = ui_token_account.map(|r| r.value.is_some()).unwrap_or(false);

            token_accounts.entry(token_address).or_insert(
                TokenAccount {
                    owner: owner_solana_address,
                    contract: contract_solana_address,
                    mint: approve.mint,
                    key: token_address,
                    new: !token_exists
                }
            );
        }
    }

    pub fn apply_erc20_approves(&self, approves: Vec<ERC20Approve>) {
        let mut solana_accounts = self.solana_accounts.borrow_mut();

        for approve in approves {
            let (address, _) = self.get_erc20_allowance_address(
                &approve.owner,
                &approve.spender,
                &approve.contract,
                &approve.mint
            );

            solana_accounts.insert(address, AccountMeta::new(address, false));
        }
    }

    pub fn get_used_accounts(&self) -> Vec<AccountJSON>
    {
        let mut arr = Vec::new();

        let accounts = self.accounts.borrow();
        for (address, acc) in accounts.iter() {
            let (solana_address, _solana_nonce) = make_solana_program_address(address, &self.config.evm_loader);

            let contract_address = {
                let addr = AccountData::unpack(&acc.account.data).unwrap().get_account().unwrap().code_account;
                if addr == Pubkey::new_from_array([0_u8; 32]) {
                    None
                } else {
                    Some(addr)
                }
            };

            if !is_precompile_address(address) {
                arr.push(AccountJSON{
                        address: "0x".to_string() + &hex::encode(&address.to_fixed_bytes()),
                        writable: acc.writable,
                        new: false,
                        account: solana_address.to_string(),
                        contract: contract_address.map(|v| v.to_string()),
                        code_size: acc.code_size,
                        code_size_current: acc.code_size_current
                });
            }
        }

        let new_accounts = self.new_accounts.borrow();
        for (address, acc) in new_accounts.iter() {
            if !is_precompile_address(address) {
                arr.push(AccountJSON{
                        address: "0x".to_string() + &hex::encode(&address.to_fixed_bytes()),
                        writable: acc.writable,
                        new: true,
                        account: acc.key.to_string(),
                        contract: None,
                        code_size: acc.code_size,
                        code_size_current : None
                });
            }
        }

        arr
    }
}

pub fn make_solana_program_address(
    ether_address: &H160,
    program_id: &Pubkey
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&[ACCOUNT_SEED_VERSION], ether_address.as_bytes()], program_id)
}

impl<'a> AccountStorage for EmulatorAccountStorage<'a> {
    fn apply_to_account<U, D, F>(&self, address: &H160, d: D, f: F) -> U
    where F: FnOnce(&SolidityAccount) -> U,
          D: FnOnce() -> U
    {
        self.create_acc_if_not_exists(address);
        let accounts = self.accounts.borrow();
        match accounts.get(address) {
            None => d(),
            Some(acc) => {
                let account_data = match AccountData::unpack(&acc.account.data) {
                    Ok(acc_data) => match acc_data {
                        AccountData::Account(_) => acc_data,
                        _ => return d(),
                    },
                    Err(_) => return d(),
                };
                if acc.code_account.is_some() {
                    let mut code_data = acc.code_account.as_ref().unwrap().data.clone();
                    let contract_data = match AccountData::unpack(&code_data) {
                        Ok(acc_data) => match acc_data {
                            AccountData::Contract(_) => acc_data,
                            _ => return d(),
                        },
                        Err(_) => return d(),
                    };
                    let code_data: std::rc::Rc<std::cell::RefCell<&mut [u8]>> = Rc::new(RefCell::new(&mut code_data));
                    let account = SolidityAccount::new(&acc.key, acc.balance, account_data, Some((contract_data, code_data)));
                    f(&account)
                } else {
                    let account = SolidityAccount::new(&acc.key, acc.balance, account_data, None);
                    f(&account)
                }
            },
        }
    }

    fn apply_to_solana_account<U, D, F>(&self, address: &Pubkey, d: D, f: F) -> U
    where F: FnOnce(/*data: */ &[u8], /*owner: */ &Pubkey) -> U,
          D: FnOnce() -> U
    {
        let mut solana_accounts = self.solana_accounts.borrow_mut();
        solana_accounts.entry(*address).or_insert_with(|| AccountMeta::new_readonly(*address, false));

        let account = self.config.rpc_client.get_account_with_commitment(address, CommitmentConfig::processed()).unwrap().value;
        match account {
            Some(account) => f(&account.data, &account.owner),
            None => d()
        }
    }

    fn program_id(&self) -> &Pubkey { &self.config.evm_loader }

    fn contract(&self) -> H160 { self.contract_id }

    fn origin(&self) -> H160 { self.caller_id }

    fn block_number(&self) -> U256 { self.block_number.into() }

    fn block_timestamp(&self) -> U256 { self.block_timestamp.into() }

    fn get_account_solana_address(&self, address: &H160) -> Pubkey {
        make_solana_program_address(address, &self.config.evm_loader).0
    }
}
