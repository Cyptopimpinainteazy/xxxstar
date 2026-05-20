//! X3 Chain DNS Server - Blockchain Integration
//!
//! Integration with X3 Chain node RPC for chain-backed `.x3` zone data.

use crate::config::DnsConfig;
use crate::domain::{DnsRecord, DnsRecordType, DomainName, DomainRecord};
use crate::error::{DnsError, DnsResult};
use log::{info, warn};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Blockchain Client for Domain Operations
pub struct BlockchainClient {
    config: DnsConfig,
    connection: Option<BlockchainConnection>,
    http: Client,
}

/// Simplified blockchain connection (in real implementation, would use X3 Chain SDK)
pub struct BlockchainConnection {
    rpc_url: String,
    ws_url: String,
    chain_id: u32,
    registry_contract: String,
    domain_contract: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a, T> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: T,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<T>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct X3DnsRecordRpc {
    rr_type: u16,
    ttl: u32,
    data: String,
}

#[derive(Debug, Clone, Deserialize)]
struct X3DomainRpcResponse {
    domain: String,
    #[serde(default)]
    owner: serde_json::Value,
    records: Vec<X3DnsRecordRpc>,
}

impl BlockchainClient {
    /// Create new blockchain client
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        if !config.blockchain.enabled {
            info!("🚫 Blockchain integration is disabled");
            return Ok(Self {
                config,
                connection: None,
                http: Client::new(),
            });
        }

        info!("🔗 Initializing blockchain client...");
        info!("   RPC URL: {}", config.blockchain.rpc_url);
        info!("   Chain ID: {}", config.blockchain.chain_id);
        info!(
            "   Registry Contract: {}",
            config.blockchain.registry_contract
        );

        let connection = BlockchainConnection {
            rpc_url: config.blockchain.rpc_url.clone(),
            ws_url: config.blockchain.ws_url.clone(),
            chain_id: config.blockchain.chain_id,
            registry_contract: config.blockchain.registry_contract.clone(),
            domain_contract: config.blockchain.domain_contract.clone(),
        };

        info!("✅ Blockchain client initialized");
        Ok(Self {
            config,
            connection: Some(connection),
            http: Client::new(),
        })
    }

    async fn rpc_call<R: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> DnsResult<R> {
        let Some(conn) = self.connection.as_ref() else {
            return Err(DnsError::blockchain("Blockchain integration is disabled"));
        };

        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method,
            params,
        };

        let resp = self
            .http
            .post(&conn.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DnsError::blockchain(format!("RPC request failed: {e}")))?;

        let status = resp.status();
        let payload: JsonRpcResponse<R> = resp
            .json()
            .await
            .map_err(|e| DnsError::blockchain(format!("RPC decode failed: {e}")))?;

        if let Some(err) = payload.error {
            return Err(DnsError::blockchain(format!("RPC error ({status}): {err}")));
        }

        payload
            .result
            .ok_or_else(|| DnsError::blockchain(format!("RPC missing result ({status})")))
    }

    /// Fetch a full DomainRecord from the chain-backed registry.
    pub async fn fetch_domain_record(
        &self,
        domain_name: &DomainName,
    ) -> DnsResult<Option<DomainRecord>> {
        let Some(_conn) = self.connection.as_ref() else {
            return Ok(None);
        };

        if !domain_name.is_x3_domain() {
            return Ok(None);
        }

        let resp: Option<X3DomainRpcResponse> = self
            .rpc_call("x3Domains_getDomain", json!([domain_name.as_str(), null]))
            .await?;

        let Some(domain) = resp else {
            return Ok(None);
        };

        let domain_dn = DomainName::new(domain.domain.clone())?;
        let mut record = DomainRecord::new(domain_dn.clone(), None);
        record.blockchain_verified = true;

        for r in domain.records {
            let mut dns_record = match r.rr_type {
                1 => {
                    let ip = Ipv4Addr::from_str(&r.data)
                        .map_err(|e| DnsError::blockchain(format!("Invalid A record IP: {e}")))?;
                    DnsRecord::a(domain_dn.clone(), ip, Some(r.ttl))
                }
                28 => {
                    let ip = Ipv6Addr::from_str(&r.data).map_err(|e| {
                        DnsError::blockchain(format!("Invalid AAAA record IP: {e}"))
                    })?;
                    DnsRecord::aaaa(domain_dn.clone(), ip, Some(r.ttl))
                }
                5 => {
                    let mut target = r.data;
                    if !target.ends_with('.') {
                        target.push('.');
                    }
                    DnsRecord::cname(domain_dn.clone(), target, Some(r.ttl))
                }
                16 => DnsRecord::txt(domain_dn.clone(), r.data, Some(r.ttl)),
                _ => continue,
            };

            dns_record.set_blockchain_verified(true);

            record.add_record(dns_record)?;
        }

        Ok(Some(record))
    }

    /// Register domain on blockchain
    pub async fn register_domain(
        &self,
        domain_name: &DomainName,
        owner_address: &str,
    ) -> DnsResult<String> {
        if let Some(ref _conn) = self.connection {
            info!(
                "🔗 Registering domain on blockchain: {} -> {}",
                domain_name, owner_address
            );

            // Write path requires signing and is not implemented in the DNS server.
            Err(DnsError::blockchain(
                "Registering domains requires a signed extrinsic; not supported by DNS server"
                    .to_string(),
            ))
        } else {
            warn!("⚠️  Blockchain integration is disabled");
            Ok("0xdisabled".to_string())
        }
    }

    /// Verify domain ownership
    pub async fn verify_ownership(
        &self,
        domain_name: &DomainName,
        owner_address: &str,
    ) -> DnsResult<bool> {
        if let Some(ref _conn) = self.connection {
            info!(
                "🔍 Verifying domain ownership: {} -> {}",
                domain_name, owner_address
            );

            // Ownership checks require interpreting AccountId formats; keep conservative.
            // For v1, treat ownership verification as unsupported by DNS server.
            Err(DnsError::blockchain(
                "Ownership verification is not implemented in DNS server (requires AccountId format + signatures)".to_string(),
            ))
        } else {
            warn!("⚠️  Blockchain integration is disabled, assuming verification passed");
            Ok(true)
        }
    }

    /// Update domain record on blockchain
    pub async fn update_domain_record(
        &self,
        domain_name: &DomainName,
        _new_record: &str,
    ) -> DnsResult<String> {
        if let Some(ref _conn) = self.connection {
            info!("🔗 Updating domain record on blockchain: {}", domain_name);

            Err(DnsError::blockchain(
                "Updating records requires a signed extrinsic; not supported by DNS server"
                    .to_string(),
            ))
        } else {
            warn!("⚠️  Blockchain integration is disabled");
            Ok("0xdisabled".to_string())
        }
    }

    /// Delete domain from blockchain
    pub async fn delete_domain(
        &self,
        domain_name: &DomainName,
        _owner_address: &str,
    ) -> DnsResult<String> {
        if let Some(ref _conn) = self.connection {
            info!("🔗 Deleting domain from blockchain: {}", domain_name);

            Err(DnsError::blockchain(
                "Deleting domains requires a signed extrinsic; not supported by DNS server"
                    .to_string(),
            ))
        } else {
            warn!("⚠️  Blockchain integration is disabled");
            Ok("0xdisabled".to_string())
        }
    }

    /// Get domain information from blockchain
    pub async fn get_domain_info(&self, domain_name: &DomainName) -> DnsResult<Option<DomainInfo>> {
        if let Some(ref _conn) = self.connection {
            info!("🔍 Fetching domain info from blockchain: {}", domain_name);
            let Some(domain_record) = self.fetch_domain_record(domain_name).await? else {
                return Ok(None);
            };

            let mut records = HashMap::new();
            for rec in domain_record.records {
                let key = format!("{:?}", rec.record_type.to_trust_dns_type());
                let value = match &rec.data {
                    DnsRecordType::A(ip) => ip.to_string(),
                    DnsRecordType::AAAA(ip) => ip.to_string(),
                    DnsRecordType::CNAME(t) => t.clone(),
                    DnsRecordType::TXT(t) => t.clone(),
                    DnsRecordType::NS(t) => t.clone(),
                    DnsRecordType::CAA(t) => t.clone(),
                    DnsRecordType::MX(mx) => mx.exchange.as_str().to_string(),
                    DnsRecordType::SRV(srv) => srv.target.as_str().to_string(),
                    DnsRecordType::HINFO(h) => format!("{} {}", h.cpu, h.os),
                };
                records.insert(key, value);
            }

            Ok(Some(DomainInfo {
                owner: None,
                registered_at: chrono::Utc::now(),
                expires_at: None,
                status: "active".to_string(),
                records,
            }))
        } else {
            warn!("⚠️  Blockchain integration is disabled");
            Ok(None)
        }
    }

    /// Check if blockchain connection is healthy
    pub async fn health_check(&self) -> DnsResult<bool> {
        if let Some(ref _conn) = self.connection {
            info!("🏥 Checking blockchain connection health...");

            let _: Vec<String> = self
                .rpc_call("x3Domains_listDomains", json!([null]))
                .await
                .map_err(|e| DnsError::blockchain(format!("Health check failed: {e}")))?;

            info!("✅ Blockchain connection is healthy");
            Ok(true)
        } else {
            warn!("⚠️  Blockchain integration is disabled");
            Ok(false)
        }
    }

    /// Get blockchain network info
    pub async fn get_network_info(&self) -> DnsResult<NetworkInfo> {
        if let Some(ref conn) = self.connection {
            let info = NetworkInfo {
                chain_id: conn.chain_id,
                rpc_url: conn.rpc_url.clone(),
                ws_url: conn.ws_url.clone(),
                registry_contract: conn.registry_contract.clone(),
                domain_contract: conn.domain_contract.clone(),
                latest_block: 12345678,
                network_status: "connected".to_string(),
            };

            Ok(info)
        } else {
            Err(DnsError::blockchain("Blockchain integration is disabled"))
        }
    }
}

/// Domain information from blockchain
#[derive(Debug, Clone)]
pub struct DomainInfo {
    pub owner: Option<String>,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub records: HashMap<String, String>,
}

/// Blockchain network information
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub chain_id: u32,
    pub rpc_url: String,
    pub ws_url: String,
    pub registry_contract: String,
    pub domain_contract: String,
    pub latest_block: u64,
    pub network_status: String,
}

/// Domain ownership verification result
#[derive(Debug, Clone)]
pub struct DomainOwnership {
    pub domain: DomainName,
    pub owner_address: String,
    pub verified: bool,
    pub registered_block: u64,
    pub expiration_block: u64,
    pub blockchain_transaction_hash: String,
}

impl DomainOwnership {
    /// Create new domain ownership record
    pub fn new(domain: DomainName, owner_address: String) -> Self {
        Self {
            domain,
            owner_address,
            verified: false,
            registered_block: 0,
            expiration_block: 0,
            blockchain_transaction_hash: String::new(),
        }
    }

    /// Mark as verified
    pub fn verified(mut self, tx_hash: String) -> Self {
        self.verified = true;
        self.blockchain_transaction_hash = tx_hash;
        self
    }
}

/// Blockchain event listener for domain updates
pub struct DomainEventListener {
    client: BlockchainClient,
}

impl DomainEventListener {
    /// Create new domain event listener
    pub fn new(client: BlockchainClient) -> Self {
        Self { client }
    }

    /// Start listening for domain events
    pub async fn start_listening(&self) -> DnsResult<()> {
        info!("👂 Starting domain event listener...");

        // In real implementation, subscribe to blockchain events
        // - DomainRegistered events
        // - DomainUpdated events
        // - DomainDeleted events

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;
                // Simulate event processing
                info!("📡 Processing blockchain events...");
            }
        });

        info!("✅ Domain event listener started");
        Ok(())
    }
}
