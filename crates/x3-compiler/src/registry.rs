// DEX/bridge registry for asset validation

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DEXEntry {
    pub name: String,
    pub chain: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeEntry {
    pub name: String,
    pub chains: Vec<String>,
    pub supported_assets: Vec<String>,
}

pub struct Registry {
    dexes: Vec<DEXEntry>,
    bridges: Vec<BridgeEntry>,
}

impl Registry {
    pub fn new() -> Self {
        let mut registry = Registry {
            dexes: Vec::new(),
            bridges: Vec::new(),
        };

        // Sample DEX entries
        registry.dexes.push(DEXEntry {
            name: "Uniswap v3".to_string(),
            chain: "Ethereum".to_string(),
            symbols: vec!["USDC".to_string(), "WETH".to_string(), "WBTC".to_string()],
        });

        registry.dexes.push(DEXEntry {
            name: "PancakeSwap".to_string(),
            chain: "BSC".to_string(),
            symbols: vec!["USDT".to_string(), "BNB".to_string()],
        });

        // Sample bridge entries
        registry.bridges.push(BridgeEntry {
            name: "Wormhole".to_string(),
            chains: vec!["Ethereum".to_string(), "Solana".to_string()],
            supported_assets: vec!["USDC".to_string(), "ETH".to_string()],
        });

        registry.bridges.push(BridgeEntry {
            name: "LayerZero".to_string(),
            chains: vec!["Ethereum".to_string(), "Polygon".to_string()],
            supported_assets: vec!["USDC".to_string(), "MATIC".to_string()],
        });

        registry
    }

    pub fn is_valid_symbol(&self, symbol: &str) -> bool {
        self.dexes
            .iter()
            .any(|dex| dex.symbols.contains(&symbol.to_string()))
            || self
                .bridges
                .iter()
                .any(|bridge| bridge.supported_assets.contains(&symbol.to_string()))
    }

    pub fn get_symbol_info(&self, symbol: &str) -> Option<(String, String)> {
        for dex in &self.dexes {
            if dex.symbols.contains(&symbol.to_string()) {
                return Some((dex.name.clone(), dex.chain.clone()));
            }
        }
        for bridge in &self.bridges {
            if bridge.supported_assets.contains(&symbol.to_string()) {
                return Some((bridge.name.clone(), bridge.chains[0].clone()));
            }
        }
        None
    }
}
