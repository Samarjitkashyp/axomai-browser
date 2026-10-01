//! Web3 Native Multi-Chain Crypto Wallet & dApp Provider for Axomai Browser.
//! Injects standard `window.ethereum` and `window.solana` providers into web contexts without external extensions.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedChain {
    EthereumMainnet,
    Polygon,
    Arbitrum,
    SolanaMainnet,
}

impl SupportedChain {
    pub fn chain_id_hex(&self) -> &'static str {
        match self {
            SupportedChain::EthereumMainnet => "0x1",
            SupportedChain::Polygon => "0x89",
            SupportedChain::Arbitrum => "0xa4b1",
            SupportedChain::SolanaMainnet => "solana:mainnet",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalletAccount {
    pub address: String,
    pub is_connected: bool,
}

pub struct Web3Wallet {
    pub active_chain: SupportedChain,
    pub accounts: Vec<WalletAccount>,
    pub connected_origins: HashMap<String, bool>,
}

impl Web3Wallet {
    pub fn new() -> Self {
        Web3Wallet {
            active_chain: SupportedChain::EthereumMainnet,
            accounts: vec![WalletAccount {
                address: "0x71C7656EC7ab88b098defB751B7401B5f6d8976F".to_string(),
                is_connected: true,
            }],
            connected_origins: HashMap::new(),
        }
    }

    pub fn handle_rpc(&mut self, origin: &str, method: &str, _params: &str) -> Result<String, String> {
        match method {
            "eth_requestAccounts" | "eth_accounts" => {
                self.connected_origins.insert(origin.to_string(), true);
                let addresses: Vec<String> = self.accounts.iter().map(|a| format!("\"{}\"", a.address)).collect();
                Ok(format!("[{}]", addresses.join(",")))
            }
            "eth_chainId" => {
                Ok(format!("\"{}\"", self.active_chain.chain_id_hex()))
            }
            "personal_sign" => {
                // Synthetic EIP-191 signature
                Ok(r#""0x54321deadbeefcafebabe0000000000000000000000000000000000000000000001b""#.to_string())
            }
            _ => Err(format!("Unsupported Web3 RPC method: {}", method)),
        }
    }

    pub fn switch_chain(&mut self, chain: SupportedChain) {
        self.active_chain = chain;
    }
}
