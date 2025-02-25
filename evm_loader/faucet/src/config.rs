//! Faucet config module.

use std::env;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tracing::warn;

use solana_sdk::signer::keypair::Keypair;

use crate::ethereum;

lazy_static::lazy_static! {
    static ref CONFIG: RwLock<Faucet> = RwLock::new(Faucet::default());
}

pub const DEFAULT_CONFIG: &str = "faucet.conf";
pub const AUTO: &str = "auto";

/// Represents the config errors.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to read file '{1}': {0}")]
    Read(#[source] std::io::Error, PathBuf),

    #[error("Failed to parse config '{1}': {0}")]
    Parse(#[source] toml::de::Error, PathBuf),

    #[error("Failed to parse boolean literal from config")]
    ParseBool(#[from] std::str::ParseBoolError),

    #[error("Failed to parse integer number from config")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Invalid keypair '{0}' from file '{1}'")]
    InvalidKeypair(String, PathBuf),

    #[error("Failed to parse keypair")]
    ParseKeypair(#[from] ed25519_dalek::SignatureError),
}

/// Represents the config result type.
pub type Result<T> = std::result::Result<T, Error>;

const FAUCET_RPC_PORT: &str = "FAUCET_RPC_PORT";
const FAUCET_RPC_ALLOWED_ORIGINS: &str = "FAUCET_RPC_ALLOWED_ORIGINS";
const FAUCET_WEB3_ENABLE: &str = "FAUCET_WEB3_ENABLE";
const WEB3_RPC_URL: &str = "WEB3_RPC_URL";
const WEB3_PRIVATE_KEY: &str = "WEB3_PRIVATE_KEY";
const NEON_ERC20_TOKENS: &str = "NEON_ERC20_TOKENS";
const NEON_ERC20_MAX_AMOUNT: &str = "NEON_ERC20_MAX_AMOUNT";
const FAUCET_SOLANA_ENABLE: &str = "FAUCET_SOLANA_ENABLE";
const SOLANA_URL: &str = "SOLANA_URL";
const EVM_LOADER: &str = "EVM_LOADER";
const NEON_TOKEN_MINT: &str = "NEON_TOKEN_MINT";
const NEON_TOKEN_MINT_DECIMALS: &str = "NEON_TOKEN_MINT_DECIMALS";
const NEON_OPERATOR_KEYFILE: &str = "NEON_OPERATOR_KEYFILE";
const NEON_ETH_MAX_AMOUNT: &str = "NEON_ETH_MAX_AMOUNT";
static ENV: &[&str] = &[
    FAUCET_RPC_PORT,
    FAUCET_RPC_ALLOWED_ORIGINS,
    FAUCET_WEB3_ENABLE,
    WEB3_RPC_URL,
    WEB3_PRIVATE_KEY,
    NEON_ERC20_TOKENS,
    NEON_ERC20_MAX_AMOUNT,
    FAUCET_SOLANA_ENABLE,
    SOLANA_URL,
    EVM_LOADER,
    NEON_TOKEN_MINT,
    NEON_TOKEN_MINT_DECIMALS,
    NEON_OPERATOR_KEYFILE,
    NEON_ETH_MAX_AMOUNT,
];

/// Reports if no file exists (it's normal, will be another source of config).
pub fn check_file_exists(file: &Path) {
    if !file.exists() {
        warn!(
            "File {:?} is missing; environment variables will be used",
            file
        );
    }
}

/// Shows the environment variables and their values.
pub fn show_env() {
    for e in ENV {
        let val = env::var(e).unwrap_or_else(|_| " <undefined>".into());
        println!("{}={}", e, val);
    }
}

/// Loads the config from a file and applies defined environment variables.
pub fn load(filename: &Path) -> Result<()> {
    if filename.exists() {
        CONFIG.write().unwrap().load(filename)?;
    }

    for e in ENV {
        if let Ok(val) = env::var(e) {
            match *e {
                FAUCET_RPC_PORT => CONFIG.write().unwrap().rpc.port = val.parse::<u16>()?,
                FAUCET_RPC_ALLOWED_ORIGINS => {
                    CONFIG.write().unwrap().rpc.allowed_origins = split_comma_separated_list(&val)
                }
                FAUCET_WEB3_ENABLE => CONFIG.write().unwrap().web3.enable = val.parse::<bool>()?,
                WEB3_RPC_URL => CONFIG.write().unwrap().web3.rpc_url = val,
                WEB3_PRIVATE_KEY => CONFIG.write().unwrap().web3.private_key = val,
                NEON_ERC20_TOKENS => {
                    CONFIG.write().unwrap().web3.tokens = split_comma_separated_list(&val)
                }
                NEON_ERC20_MAX_AMOUNT => {
                    CONFIG.write().unwrap().web3.max_amount = val.parse::<u64>()?
                }
                FAUCET_SOLANA_ENABLE => {
                    CONFIG.write().unwrap().solana.enable = val.parse::<bool>()?
                }
                SOLANA_URL => CONFIG.write().unwrap().solana.url = val,
                EVM_LOADER => CONFIG.write().unwrap().solana.evm_loader = val,
                NEON_TOKEN_MINT => CONFIG.write().unwrap().solana.token_mint = val,
                NEON_TOKEN_MINT_DECIMALS => {
                    CONFIG.write().unwrap().solana.token_mint_decimals = val.parse::<u8>()?
                }
                NEON_OPERATOR_KEYFILE => {
                    CONFIG.write().unwrap().solana.operator_keyfile = val.into()
                }
                NEON_ETH_MAX_AMOUNT => {
                    CONFIG.write().unwrap().solana.max_amount = val.parse::<u64>()?
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}

/// Shows the current config.
pub fn show() {
    println!("{}", CONFIG.read().unwrap())
}

/// Gets the `rpc.port` value.
pub fn rpc_port() -> u16 {
    CONFIG.read().unwrap().rpc.port
}

/// Gets the CORS `rpc.allowed_origins` urls.
pub fn allowed_origins() -> Vec<String> {
    CONFIG.read().unwrap().rpc.allowed_origins.clone()
}

/// Gets the `web3.enable` value.
pub fn web3_enabled() -> bool {
    CONFIG.read().unwrap().web3.enable
}

/// Gets the `web3.rpc_url` value.
pub fn web3_rpc_url() -> String {
    CONFIG.read().unwrap().web3.rpc_url.clone()
}

/// Gets the `web3.private_key` value. Removes prefix 0x if any.
pub fn web3_private_key() -> String {
    let key = &CONFIG.read().unwrap().web3.private_key;
    ethereum::strip_0x_prefix(key).to_owned()
}

/// Gets the `web3.tokens` addresses.
pub fn tokens() -> Vec<String> {
    CONFIG.read().unwrap().web3.tokens.clone()
}

/// Gets the `web3.max_amount` value.
pub fn web3_max_amount() -> u64 {
    CONFIG.read().unwrap().web3.max_amount
}

/// Gets the `solana.enable` value.
pub fn solana_enabled() -> bool {
    CONFIG.read().unwrap().solana.enable
}

/// Gets the `solana.url` value.
pub fn solana_url() -> String {
    CONFIG.read().unwrap().solana.url.clone()
}

/// Gets the `solana.evm_loader` address value.
pub fn solana_evm_loader() -> String {
    CONFIG.read().unwrap().solana.evm_loader.clone()
}

/// Gets the `solana.token_mint` address value.
pub fn solana_token_mint_id() -> String {
    CONFIG.read().unwrap().solana.token_mint.clone()
}

/// Gets the `solana.token_mint_decimals` value.
pub fn solana_token_mint_decimals() -> u8 {
    CONFIG.read().unwrap().solana.token_mint_decimals
}

/// Gets the `solana.operator` keypair value.
pub fn solana_operator_keypair() -> Result<Keypair> {
    let keyfile = CONFIG.read().unwrap().solana.operator_keyfile.clone();
    let key = std::fs::read_to_string(&keyfile).map_err(|e| Error::Read(e, keyfile.clone()))?;
    let key = key.trim();
    if !(key.starts_with('[') && key.ends_with(']')) {
        return Err(Error::InvalidKeypair(key.into(), keyfile));
    }
    let ss = split_comma_separated_list(trim_first_and_last_chars(key));
    let mut bytes = Vec::with_capacity(ss.len());
    for s in ss {
        bytes.push(s.parse::<u8>()?);
    }
    Ok(Keypair::from_bytes(&bytes)?)
}

/// Gets the `solana.max_amount` value
pub fn solana_max_amount() -> u64 {
    CONFIG.read().unwrap().solana.max_amount
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct Rpc {
    port: u16,
    allowed_origins: Vec<String>,
}

impl std::fmt::Display for Rpc {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "rpc.port = {}", self.port)?;
        if env::var(FAUCET_RPC_PORT).is_ok() {
            writeln!(f, " (overridden by {})", FAUCET_RPC_PORT)?;
        } else {
            writeln!(f)?;
        }
        write!(f, "rpc.allowed_origins = {:?}", self.allowed_origins)?;
        if env::var(FAUCET_RPC_ALLOWED_ORIGINS).is_ok() {
            write!(f, " (overridden by {})", FAUCET_RPC_ALLOWED_ORIGINS)
        } else {
            write!(f, "")
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct Web3 {
    enable: bool,
    rpc_url: String,
    private_key: String,
    tokens: Vec<String>,
    max_amount: u64,
}

impl std::fmt::Display for Web3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "web3.enable = {}", self.enable)?;
        if env::var(FAUCET_WEB3_ENABLE).is_ok() {
            write!(f, " (overridden by {})", FAUCET_WEB3_ENABLE)?;
        } else {
            write!(f, "")?;
        }
        if !self.enable {
            return Ok(());
        }
        writeln!(f)?;
        write!(f, "web3.rpc_url = {}", self.rpc_url)?;
        if env::var(WEB3_RPC_URL).is_ok() {
            writeln!(f, " (overridden by {})", WEB3_RPC_URL)?;
        } else {
            writeln!(f)?;
        }
        write!(
            f,
            "web3.private_key = {}",
            obfuscate_string(&self.private_key)
        )?;
        if env::var(WEB3_PRIVATE_KEY).is_ok() {
            writeln!(f, " (overridden by {})", WEB3_PRIVATE_KEY)?;
        } else {
            writeln!(f)?;
        }
        write!(
            f,
            "web3.tokens = {:?}",
            obfuscate_list_of_strings(&self.tokens)
        )?;
        if env::var(NEON_ERC20_TOKENS).is_ok() {
            writeln!(f, " (overridden by {})", NEON_ERC20_TOKENS)?;
        } else {
            writeln!(f)?;
        }
        write!(f, "web3.max_amount = {}", self.max_amount)?;
        if env::var(NEON_ERC20_MAX_AMOUNT).is_ok() {
            write!(f, " (overridden by {})", NEON_ERC20_MAX_AMOUNT)
        } else {
            write!(f, "")
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct Solana {
    enable: bool,
    url: String,
    evm_loader: String,
    token_mint: String,
    token_mint_decimals: u8,
    operator_keyfile: PathBuf,
    max_amount: u64,
}

impl std::fmt::Display for Solana {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "solana.enable = {}", self.enable)?;
        if env::var(FAUCET_SOLANA_ENABLE).is_ok() {
            write!(f, " (overridden by {})", FAUCET_SOLANA_ENABLE)?;
        } else {
            write!(f, "")?;
        }
        if !self.enable {
            return Ok(());
        }
        writeln!(f)?;
        write!(f, "solana.url = {}", self.url)?;
        if env::var(SOLANA_URL).is_ok() {
            writeln!(f, " (overridden by {})", SOLANA_URL)?;
        } else {
            writeln!(f)?;
        }
        write!(
            f,
            "solana.evm_loader = {:?}",
            obfuscate_string(&self.evm_loader)
        )?;
        if env::var(EVM_LOADER).is_ok() {
            writeln!(f, " (overridden by {})", EVM_LOADER)?;
        } else {
            writeln!(f)?;
        }
        write!(
            f,
            "solana.token_mint = {:?}",
            obfuscate_string(&self.token_mint)
        )?;
        if env::var(NEON_TOKEN_MINT).is_ok() {
            writeln!(f, " (overridden by {})", NEON_TOKEN_MINT)?;
        } else {
            writeln!(f)?;
        }
        write!(
            f,
            "solana.token_mint_decimals = {}",
            self.token_mint_decimals
        )?;
        if env::var(NEON_TOKEN_MINT_DECIMALS).is_ok() {
            writeln!(f, " (overridden by {})", NEON_TOKEN_MINT_DECIMALS)?;
        } else {
            writeln!(f)?;
        }
        write!(f, "solana.operator_keyfile = {:?}", self.operator_keyfile)?;
        if env::var(NEON_OPERATOR_KEYFILE).is_ok() {
            writeln!(f, " (overridden by {})", NEON_OPERATOR_KEYFILE)?;
        } else {
            writeln!(f)?;
        }
        write!(f, "solana.max_amount = {}", self.max_amount)?;
        if env::var(NEON_ETH_MAX_AMOUNT).is_ok() {
            write!(f, " (overridden by {})", NEON_ETH_MAX_AMOUNT)
        } else {
            write!(f, "")
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct Faucet {
    rpc: Rpc,
    web3: Web3,
    solana: Solana,
}

impl Faucet {
    /// Constructs config from a file.
    fn load(&mut self, filename: &Path) -> Result<()> {
        let text =
            std::fs::read_to_string(filename).map_err(|e| Error::Read(e, filename.to_owned()))?;
        *self = toml::from_str(&text).map_err(|e| Error::Parse(e, filename.to_owned()))?;
        Ok(())
    }
}

impl std::fmt::Display for Faucet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}", self.rpc)?;
        writeln!(f, "{}", self.web3)?;
        write!(f, "{}", self.solana)
    }
}

fn obfuscate_list_of_strings(keys: &[String]) -> Vec<String> {
    keys.iter().map(|s| obfuscate_string(s)).collect()
}

/// Cuts middle part of a key like `0x1234ABC`.
fn obfuscate_string(key: &str) -> String {
    let len = key.len();
    let prefix_len = if key.starts_with("0x") { 6 } else { 4 };
    let suffix_len = 4;
    if len <= prefix_len + suffix_len {
        key.into()
    } else {
        format!("{}•••{}", &key[..prefix_len], &key[len - suffix_len..])
    }
}

/// Cuts middle part of a key like `[1,2,3...N]`.
#[allow(unused)]
fn obfuscate_solana_private_key(key: &str) -> String {
    let ss = split_comma_separated_list(key);
    let len = ss.len();
    if len <= 8 {
        key.into()
    } else {
        format!(
            "{},{},{},{}•••{},{},{},{}",
            ss[0],
            ss[1],
            ss[2],
            ss[3],
            ss[len - 4],
            ss[len - 3],
            ss[len - 2],
            ss[len - 1]
        )
    }
}

#[test]
fn test_obfuscate() {
    let s = obfuscate_string("123");
    assert_eq!(s, "123");
    let s = obfuscate_string("123456789");
    assert_eq!(s, "1234•••6789");
    let s = obfuscate_string("0x123456789");
    assert_eq!(s, "0x1234•••6789");

    let s = obfuscate_list_of_strings(&vec!["AAA".to_string(), "BBB".to_string()]);
    assert_eq!(s, vec!["AAA", "BBB"]);
    let s = obfuscate_list_of_strings(&vec!["CCCCCCCCC".to_string(), "DDDDDDDDD".to_string()]);
    assert_eq!(s, vec!["CCCC•••CCCC", "DDDD•••DDDD"]);

    let s = obfuscate_solana_private_key("123");
    assert_eq!(s, "123");
    let s = obfuscate_solana_private_key("1,2,3");
    assert_eq!(s, "1,2,3");
    let s = obfuscate_solana_private_key("1,2,3,4,5,6,7,8");
    assert_eq!(s, "1,2,3,4,5,6,7,8");
    let s = obfuscate_solana_private_key("1,2,3,4,5,6,7,8,9");
    assert_eq!(s, "1,2,3,4•••6,7,8,9");
}

/// Splits string as comma-separated list and trims whitespace.
/// String `"A ,B, C    "` will produce vector `["A","B","C"]`.
fn split_comma_separated_list(s: &str) -> Vec<String> {
    s.split(',').map(|s| s.trim().to_owned()).collect()
}

#[test]
fn test_split_comma_separated_list() {
    let ss = split_comma_separated_list("".into());
    assert_eq!(ss, vec!(""));
    let ss = split_comma_separated_list("ABC".into());
    assert_eq!(ss, vec!("ABC"));
    let ss = split_comma_separated_list("ABC,DEF".into());
    assert_eq!(ss, vec!("ABC", "DEF"));
    let ss = split_comma_separated_list("ABC,DEF,GHI".into());
    assert_eq!(ss, vec!("ABC", "DEF", "GHI"));
    let ss = split_comma_separated_list("ABC,".into());
    assert_eq!(ss, vec!("ABC", ""));
    let ss = split_comma_separated_list("ABC,,".into());
    assert_eq!(ss, vec!("ABC", "", ""));
    let ss = split_comma_separated_list(",ABC".into());
    assert_eq!(ss, vec!("", "ABC"));
    let ss = split_comma_separated_list("  ,  ,  ABC".into());
    assert_eq!(ss, vec!("", "", "ABC"));
    let ss = split_comma_separated_list("   ABC   ,   DEF   ,   GHI   ".into());
    assert_eq!(ss, vec!("ABC", "DEF", "GHI"));
}

/// Returns string without it's first and last characters.
/// Works with multi-byte characters and empty strings.
fn trim_first_and_last_chars(value: &str) -> &str {
    let mut chars = value.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

#[test]
fn test_trim_first_and_last_chars() {
    let s = trim_first_and_last_chars("");
    assert!(s.is_empty());
    let s = trim_first_and_last_chars("A");
    assert!(s.is_empty());
    let s = trim_first_and_last_chars("AB");
    assert!(s.is_empty());
    let s = trim_first_and_last_chars("ABC");
    assert_eq!(s, "B");
    let s = trim_first_and_last_chars("语言处理");
    assert_eq!(s, "言处");
}
