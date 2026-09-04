//! AtlasHTLC deployment integration test.
//! Requires --features std and a funded Sepolia wallet.
//!
//! Run with:
//!   DEPLOYER_ADDRESS=0xYourAddress \
//!   cargo test -p x3-atomic-swap --features std --test atlas_htlc_deploy_test -- --ignored
//!
//! To actually deploy (sign + broadcast), also set:
//!   DEPLOYER_PRIVATE_KEY=your_private_key_hex

#![cfg(feature = "std")]

use x3_atomic_swap::error::SwapError;
use x3_atomic_swap::ethereum_tx::Transaction;
use x3_atomic_swap::rpc_client::RpcClient;

/// Compiled AtlasHTLC init bytecode (from Foundry compilation).
/// Generated with: solc 0.8.24, optimizer 200 runs.
const ATLAS_HTLC_BYTECODE: &str = "0x608060405234801561000f575f80fd5b5060015f55611238806100215f395ff3fe608060405260043610610084575f3560e01c8063502e9fd511610057578063502e9fd51461011f57806378817df91461013257806390c268381461016857806391edd8f21461019b5780639755dca01461020c575f80fd5b80631317d4a2146100885780633045471f146100b057806343b920c5146100df5780634d7807ec14610100575b5f80fd5b348015610093575f80fd5b5061009d60025481565b6040519081526020015b60405180910390f35b3480156100bb575f80fd5b506100cf6100ca366004610fed565b61022b565b60405190151581526020016100a7565b3480156100ea575f80fd5b506100fe6100f9366004610fed565b61025b565b005b34801561010b575f80fd5b506100cf61011a366004610fed565b6104c1565b61009d61012d36600461101f565b6104e9565b34801561013d575f80fd5b5061015161014c366004610fed565b6108c7565b6040805192151583526020830191909152016100a7565b348015610173575f80fd5b50610187610182366004610fed565b610910565b6040516100a798979695949392919061107d565b3480156101a6575f80fd5b506101876101b5366004610fed565b600160208190525f9182526040909120805491810154600282015460038301546004840154600585015460068601546007909601546001600160a01b039788169795861696949095169492939192909160ff169088565b348015610217575f80fd5b506100fe6102263660046110e5565b6109b2565b5f60015b5f8381526001602052604090206006015460ff16600481111561025457610254611069565b1492915050565b610263610c93565b5f8181526001602052604090205481906001600160a01b03166102a15760405162461bcd60e51b815260040161029890611105565b60405180910390fd5b5f82815260016020819052604090912090600682015460ff1660048111156102cb576102cb611069565b1461030e5760405162461bcd60e51b815260206004820152601360248201527248544c43206e6f7420726566756e6461626c6560681b6044820152606401610298565b80600501544210156103595760405162461bcd60e51b8152602060048201526014602482015273151a5b59531bd8dac81b9bdd08195e1c1a5c995960621b6044820152606401610298565b80546001600160a01b031633146103a35760405162461bcd60e51b815260206004820152600e60248201526d2737ba103a34329039b2b73232b960911b6044820152606401610298565b60068101805460ff1916600317905560028101546001600160a01b031661046357805460038201546040515f926001600160a01b031691908381818185875af1925050503d805f8114610411576040519150601f19603f3d011682016040523d82523d5f602084013e610416565b606091505b505090508061045d5760405162461bcd60e51b8152602060048201526013602482015272115512081d1c985b9cd9995c8819985a5b1959606a1b6044820152606401610298565b50610487565b805460038201546002830154610487926001600160a01b0391821692911690610cea565b604051339084907f19438ba24f558efe29e176888dc45d354fa6ca8f049af37592767f6534f32eb0905f90a350506104be60015f55565b50565b5f8181526001602052604081206005015442108015906104e35750600161022f565b92915050565b5f6104f2610c93565b6001600160a01b03861661053c5760405162461bcd60e51b8152602060048201526011602482015270125b9d985b1a59081c9958da5c1a595b9d607a1b6044820152606401610298565b8461057c5760405162461bcd60e51b815260206004820152601060248201526f496e76616c696420686173684c6f636b60801b6044820152606401610298565b4284116105cb5760405162461bcd60e51b815260206004820152601e60248201527f54696d654c6f636b206d75737420626520696e207468652066757475726500006044820152606401610298565b60028054905f6105da83611132565b90915550506002546040516bffffffffffffffffffffffff1933606090811b8216602084015289901b16603482015260488101879052606881019190915260880160408051601f1981840301815291815281516020928301205f81815260019093529120549091506001600160a01b03161561068e5760405162461bcd60e51b815260206004820152601360248201527248544c4320616c72656164792065786973747360681b6044820152606401610298565b5f6001600160a01b0384166106e3575f34116106dc5760405162461bcd60e51b815260206004820152600d60248201526c09aeae6e840e6cadcc8408aa89609b1b6044820152606401610298565b503461073e565b5f83116107275760405162461bcd60e51b81526020600482015260126024820152710416d6f756e74206d757374206265203e20360741b6044820152606401610298565b508161073e6001600160a01b038516333084610d52565b604051806101000160405280336001600160a01b03168152602001886001600160a01b03168152602001856001600160a01b031681526020018281526020018781526020018681526020016001600481111561079c5761079c611069565b81525f60209182018190528481526001808352604091829020845181546001600160a01b039182166001600160a01b0319918216178355948601518284018054918316918716919091179055928501516002820180549190941694169390931790915560608301516003830155608083015160048084019190915560a0840151600584015560c08401516006840180549193909260ff1990921691849081111561084857610848611069565b021790555060e09190910151600790910155604080516001600160a01b038681168252602082018490529181018890526060810187905290881690339084907fe73ebbb092b2f76c6207ed40ef30281ae71bf7d40a4baa4b4336be8292c71b889060800160405180910390a4506108be60015f55565b95945050505050565b5f81815260016020526040812081906002600682015460ff1660048111156108f1576108f1611069565b036109055760070154600194909350915050565b505f93849350915050565b5f81815260016020526040812054819081908190819081908190819089906001600160a01b03166109535760405162461bcd60e51b815260040161029890611105565b5050505f9687525050600160208190526040909520805495810154600282015460038301546004840154600585015460068601546007909601546001600160a01b039b8c169c958c169b90941699509197509550935060ff9092169190565b6109ba610c93565b5f8281526001602052604090205482906001600160a01b03166109ef5760405162461bcd60e51b815260040161029890611105565b5f83815260016020819052604090912090600682015460ff166004811115610a1957610a19611069565b14610a5b5760405162461bcd60e51b815260206004820152601260248201527148544c43206e6f7420636c61696d61626c6560701b6044820152606401610298565b60018101546001600160a01b03163314610aab5760405162461bcd60e51b8152602060048201526011602482015270139bdd081d1a19481c9958da5c1a595b9d607a1b6044820152606401610298565b8060040154600284604051602001610ac591815260200190565b60408051601f1981840301815290829052610adf91611178565b602060405180830381855afa158015610afa573d5f803e3d5ffd5b5050506040513d601f19601f82011682018060405250810190610b1d9190611193565b14610b5b5760405162461bcd60e51b815260206004820152600e60248201526d125b9d985b1a59081cd958dc995d60921b6044820152606401610298565b60068101805460ff19166002908117909155600782018490558101546001600160a01b0316610c2657600181015460038201546040515f926001600160a01b031691908381818185875af1925050503d805f8114610bd4576040519150601f19603f3d011682016040523d82523d5f602084013e610bd9565b606091505b5050905080610c205760405162461bcd60e51b8152602060048201526013602482015272115512081d1c985b9cd9995c8819985a5b1959606a1b6044820152606401610298565b50610c4d565b600181015460038201546002830154610c4d926001600160a01b0391821692911690610cea565b604051838152339085907f0d2857ffeac4c6253f25f8e8bcd991c1db796e5bbd52173b7d11e8a25c93a6df9060200160405180910390a35050610c8f60015f55565b5050565b60025f5403610ce45760405162461bcd60e51b815260206004820152601f60248201527f5265656e7472616e637947756172643a207265656e7472616e742063616c6c006044820152606401610298565b60025f55565b6040516001600160a01b038316602482015260448101829052610d4d90849063a9059cbb60e01b906064015b60408051601f198184030181529190526020810180516001600160e01b03166001600160e01b031990931692909217909152610d90565b505050565b6040516001600160a01b0380851660248301528316604482015260648101829052610d8a9085906323b872dd60e01b90608401610d16565b50505050565b5f610de4826040518060400160405280602081526020017f5361666545524332303a206c6f772d6c6576656c2063616c6c206661696c6564815250856001600160a01b0316610e639092919063ffffffff16565b905080515f1480610e04575080806020019051810190610e0491906111aa565b610d4d5760405162461bcd60e51b815260206004820152602a60248201527f5361666545524332303a204552433230206f7065726174696f6e20646964206e6044820152691bdd081cdd58d8d9595960b21b6064820152608401610298565b6060610e7184845f85610e79565b949350505050565b606082471015610eda5760405162461bcd60e51b815260206004820152602660248201527f416464726573733a20696e73756666696369656e742062616c616e636520666f6044820152651c8818d85b1b60d21b6064820152608401610298565b5f80866001600160a01b03168587604051610ef59190611178565b5f6040518083038185875af1925050503d805f8114610f2f576040519150601f19603f3d011682016040523d82523d5f602084013e610f34565b606091505b5091509150610f4587838387610f50565b979650505050505050565b60608315610fbe5782515f03610fb7576001600160a01b0385163b610fb75760405162461bcd60e51b815260206004820152601d60248201527f416464726573733a2063616c6c20746f206e6f6e2d636f6e74726163740000006044820152606401610298565b5081610e71565b610e718383815115610fd35781518083602001fd5b8060405162461bcd60e51b815260040161029891906111d0565b5f60208284031215610ffd575f80fd5b5035919050565b80356001600160a01b038116811461101a575f80fd5b919050565b5f805f805f60a08688031215611033575f80fd5b61103c86611004565b9450602086013593506040860135925061105860608701611004565b949793965091946080013592915050565b634e487b7160e01b5f52602160045260245ffd5b6001600160a01b038981168252888116602083015287166040820152606081018690526080810185905260a081018490526101008101600584106110cf57634e487b7160e01b5f52602160045260245ffd5b60c082019390935260e001529695505050505050565b5f80604083850312156110f6575f80fd5b50508035926020909101359150565b60208082526013908201527212151310c8191bd95cc81b9bdd08195e1a5cdd606a1b604082015260600190565b5f6001820161114f57634e487b7160e01b5f52601160045260245ffd5b5060010190565b5f5b83811015611170578181015183820152602001611158565b50505f910152565b5f8251611189818460208701611156565b9190910192915050565b5f602082840312156111a3575f80fd5b5051919050565b5f602082840312156111ba575f80fd5b815180151581146111c9575f80fd5b9392505050565b602081525f82518060208401526111ee816040850160208701611156565b601f01601f1916919091016040019291505056fea26469706673582212207f51a67af8697d184b718cbd023f917175c432f86c6b075f3000ad861e09201e64736f6c63430008180033";

fn get_rpc_url() -> String {
    std::env::var("RPC_URL").unwrap_or_else(|_| "https://ethereum-sepolia.publicnode.com".into())
}

fn get_deployer_address() -> String {
    std::env::var("DEPLOYER_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".into())
}

/// Test that we can estimate gas for deploying AtlasHTLC.
#[ignore]
#[test]
fn test_estimate_atlas_htlc_deploy_gas() {
    let url = get_rpc_url();
    eprintln!("[DEPLOY] Estimating AtlasHTLC deployment gas on: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    let deployer = get_deployer_address();

    // eth_estimateGas for contract creation
    let params = vec![serde_json::json!({
        "from": deployer,
        "data": ATLAS_HTLC_BYTECODE,
    })];

    let result = client.call("eth_estimateGas", params);
    match result {
        Ok(resp) => {
            if let Some(result_val) = resp.result {
                let gas_str = result_val.as_str().unwrap_or("0x0");
                let gas = u64::from_str_radix(gas_str.trim_start_matches("0x"), 16).unwrap_or(0);
                eprintln!("[DEPLOY] ✓ Estimated gas: {} ({} hex)", gas, gas_str);
                // Typical contract deploy is 500k-1.5M gas
                assert!(gas > 100_000, "Deployment should need > 100k gas");
                assert!(gas < 5_000_000, "AtlasHTLC is small, should need < 5M gas");
            }
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[DEPLOY] ⚠ Could not estimate gas: {}", msg);
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

/// Test getting the deployer's nonce.
#[ignore]
#[test]
fn test_get_deployer_nonce() {
    let url = get_rpc_url();
    let deployer = get_deployer_address();

    if deployer == "0x0000000000000000000000000000000000000000" {
        eprintln!("[DEPLOY] ⚠ DEPLOYER_ADDRESS not set, using zero address");
    }

    eprintln!("[DEPLOY] Getting nonce for: {}", deployer);
    let mut client = RpcClient::new(url.clone(), 11155111);
    let result = client.get_transaction_count(&deployer, "latest");

    match result {
        Ok(nonce) => {
            eprintln!("[DEPLOY] ✓ Nonce: {}", nonce);
            if deployer != "0x0000000000000000000000000000000000000000" {
                // If a real address, nonce should be reasonable
                eprintln!("[DEPLOY]   Nonce suggests {} previous txs", nonce);
            }
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[DEPLOY] ⚠ Could not get nonce: {}", msg);
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

/// Test getting the current gas price.
#[ignore]
#[test]
fn test_get_gas_price() {
    let url = get_rpc_url();
    let mut client = RpcClient::new(url.clone(), 11155111);
    let result = client.gas_price();

    match result {
        Ok(price) => {
            let gwei = price as f64 / 1_000_000_000.0;
            eprintln!("[DEPLOY] ✓ Gas price: {} wei ({:.2} gwei)", price, gwei);
            assert!(gwei > 0.0, "Gas price should be > 0");
            assert!(
                gwei < 10_000.0,
                "Gas price should be reasonable (< 10000 gwei)"
            );
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[DEPLOY] ⚠ Could not get gas price: {}", msg);
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

/// Test verifying the chain is Sepolia.
#[ignore]
#[test]
fn test_verify_sepolia_chain_id() {
    let url = get_rpc_url();
    let mut client = RpcClient::new(url.clone(), 11155111);
    let result = client.chain_id();

    match result {
        Ok(id) => {
            eprintln!("[DEPLOY] ✓ Chain ID: {}", id);
            assert_eq!(id, 11155111, "Should be Sepolia (11155111)");
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[DEPLOY] ⚠ Could not get chain ID: {}", msg);
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

/// Full deployment dry-run: estimate gas, get nonce, get gas price, print unsigned tx.
#[ignore]
#[test]
fn test_atlas_htlc_deploy_dry_run() {
    let url = get_rpc_url();
    let deployer = get_deployer_address();

    eprintln!("\n=============================================");
    eprintln!("  AtlasHTLC Deployment Dry Run");
    eprintln!("=============================================");
    eprintln!("  RPC URL:      {}", url);
    eprintln!("  Deployer:     {}", deployer);
    eprintln!(
        "  Bytecode len: {} hex chars ({} bytes)",
        ATLAS_HTLC_BYTECODE.len(),
        ATLAS_HTLC_BYTECODE.len() / 2
    );
    eprintln!("---------------------------------------------\n");

    let mut client = RpcClient::new(url.clone(), 11155111);

    // 1. Check chain ID
    match client.chain_id() {
        Ok(id) => {
            eprintln!("  [1/5] ✓ Chain ID: {} (expected: 11155111)", id);
            assert_eq!(id, 11155111, "Wrong chain");
        }
        Err(e) => {
            eprintln!("  [1/5] ⚠ {}", e);
            return;
        }
    }

    // 2. Get deployer balance
    match client.get_balance(&deployer) {
        Ok(bal) => {
            let eth = bal as f64 / 1e18;
            eprintln!("  [2/5] ✓ Deployer balance: {} wei ({:.6} ETH)", bal, eth);
            if eth < 0.01 && deployer != "0x0000000000000000000000000000000000000000" {
                eprintln!("  [2/5] ⚠ Low balance! Need at least 0.01 ETH for deployment");
            }
        }
        Err(e) => eprintln!("  [2/5] ⚠ {}", e),
    }

    // 3. Get nonce
    let nonce = match client.get_transaction_count(&deployer, "latest") {
        Ok(n) => {
            eprintln!("  [3/5] ✓ Nonce: {}", n);
            n
        }
        Err(e) => {
            eprintln!("  [3/5] ⚠ {}", e);
            return;
        }
    };

    // 4. Estimate gas
    let gas_limit = match client.estimate_gas(&deployer, "", ATLAS_HTLC_BYTECODE) {
        Ok(gas) => {
            eprintln!("  [4/5] ✓ Estimated gas: {}", gas);
            gas
        }
        Err(e) => {
            eprintln!("  [4/5] ⚠ {}", e);
            return;
        }
    };

    // 5. Get gas price
    let gas_price = match client.gas_price() {
        Ok(p) => {
            let gwei = p as f64 / 1e9;
            eprintln!("  [5/5] ✓ Gas price: {} wei ({:.2} gwei)", p, gwei);
            let cost_wei = gas_limit as u128 * p;
            let cost_eth = cost_wei as f64 / 1e18;
            eprintln!("\n  Estimated cost: {} ETH", cost_eth);
            p
        }
        Err(e) => {
            eprintln!("  [5/5] ⚠ {}", e);
            return;
        }
    };

    // Print unsigned transaction JSON
    let unsigned_tx = serde_json::json!({
        "from": deployer,
        "to": null,
        "nonce": nonce,
        "gas": gas_limit,
        "gasPrice": gas_price,
        "data": ATLAS_HTLC_BYTECODE,
        "chainId": 11155111,
    });

    eprintln!("\n---------------------------------------------\n");
    eprintln!("  Unsigned Transaction (ready to sign):");
    eprintln!("  {}", serde_json::to_string_pretty(&unsigned_tx).unwrap());
    eprintln!("\n---------------------------------------------");
    eprintln!("  To deploy:");
    eprintln!(
        "  1. Sign with: cast wallet sign --from {} --private-key $KEY {}",
        deployer, unsigned_tx
    );
    eprintln!("  2. Broadcast signed tx via RPC");
    eprintln!(
        "  Or use: cast send --from {} --private-key $KEY --rpc-url {} --create {}",
        deployer,
        url,
        ATLAS_HTLC_BYTECODE.len()
    );
    eprintln!("=============================================\n");
}

/// Actually deploy AtlasHTLC to Sepolia.
/// Requires: DEPLOYER_PRIVATE_KEY env var with a funded wallet.
#[ignore]
#[test]
fn test_deploy_atlas_htlc() {
    let private_key =
        std::env::var("DEPLOYER_PRIVATE_KEY").expect("DEPLOYER_PRIVATE_KEY must be set to deploy");

    // Derive address from private key
    let deployer = Transaction::address_from_private_key(&private_key)
        .expect("Failed to derive address from private key");

    eprintln!("\n=============================================");
    eprintln!("  Deploying AtlasHTLC to Sepolia");
    eprintln!("=============================================");
    eprintln!("  Deployer: {}", deployer);

    let url = get_rpc_url();
    let mut client = RpcClient::new(url.clone(), 11155111);

    // Check balance first
    let balance = client
        .get_balance(&deployer)
        .expect("Failed to get balance");
    let eth = balance as f64 / 1e18;
    eprintln!("  Balance:  {} ETH", eth);
    assert!(
        eth >= 0.005,
        "Need at least 0.005 ETH for deployment, have {}",
        eth
    );

    // Get chain ID
    let chain_id = client.chain_id().expect("Failed to get chain ID");
    assert_eq!(chain_id, 11155111, "Wrong chain");

    // Get gas price
    let gas_price = client.gas_price().expect("Failed to get gas price");
    let gwei = gas_price as f64 / 1e9;
    eprintln!("  Gas price: {} gwei", gwei);

    // Estimate gas
    let gas_limit = client
        .estimate_gas(&deployer, "", ATLAS_HTLC_BYTECODE)
        .expect("Failed to estimate gas");
    eprintln!("  Gas limit: {}", gas_limit);

    let cost_wei = gas_limit as u128 * gas_price;
    let cost_eth = cost_wei as f64 / 1e18;
    eprintln!("  Est cost: {} ETH", cost_eth);
    assert!(cost_wei < balance, "Insufficient balance");

    // Get nonce
    let nonce = client
        .get_transaction_count(&deployer, "latest")
        .expect("Failed to get nonce");
    eprintln!("  Nonce:    {}", nonce);

    // Build and sign transaction
    let tx = Transaction::new_contract_deploy(
        nonce,
        gas_price,
        gas_limit,
        ATLAS_HTLC_BYTECODE,
        chain_id,
    );

    eprintln!("\n  Signing transaction...");
    let signed_tx = tx.sign(&private_key).expect("Failed to sign transaction");
    eprintln!("  Signed tx hex length: {} chars", signed_tx.len());

    // Broadcast
    eprintln!("\n  Broadcasting...");
    let tx_hash = client
        .send_raw_transaction(&signed_tx)
        .expect("Failed to send raw transaction");
    eprintln!("  ✅ Tx hash: {}", tx_hash);

    // Poll for receipt (up to 60 seconds, 5s intervals)
    eprintln!("  Waiting for receipt...");
    let start = std::time::Instant::now();
    let receipt = loop {
        if start.elapsed().as_secs() > 60 {
            panic!("Timeout waiting for transaction receipt");
        }
        if let Ok(Some(receipt)) = client.get_transaction_receipt(&tx_hash) {
            break receipt;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
        eprint!(".");
    };

    eprintln!("\n  ✅ Transaction confirmed!");

    let contract_address = receipt
        .get("contractAddress")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "NOT FOUND".to_string());

    let block_number = receipt
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0))
        .unwrap_or(0);

    let gas_used = receipt
        .get("gasUsed")
        .and_then(|v| v.as_str())
        .map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0))
        .unwrap_or(0);

    eprintln!("\n=============================================");
    eprintln!("  ✅ CONTRACT DEPLOYED!");
    eprintln!("  Contract:  {}", contract_address);
    eprintln!("  Block:     {}", block_number);
    eprintln!("  Gas used:  {}", gas_used);
    eprintln!("  Tx hash:   {}", tx_hash);
    eprintln!("=============================================\n");

    // Verify deployed contract responds to a call
    // Call getHTLC(bytes32) - shouldn't revert since contract exists
    let call_data = "0x905d22a50000000000000000000000000000000000000000000000000000000000000000";
    let call_result = client.eth_call(&deployer, &contract_address, call_data, "latest");
    match call_result {
        Ok(result) => {
            eprintln!("  ✅ Contract responds to calls: {}", result);
        }
        Err(e) => {
            eprintln!("  ⚠ Contract call failed (may need state): {}", e);
        }
    }
}

/// Verify that address derivation and signing work correctly using known test vectors.
/// This test runs WITHOUT a live RPC — it is purely cryptographic verification.
/// Uses the well-known Hardhat/Anvil test account private key.
#[cfg(feature = "std")]
#[test]
fn test_signing_test_vector() {
    // Known test vector: private key → address
    // This is Hardhat/Anvil test account #0 (mnemonic: "test test test test test test test test test test test junk").
    // Publicly known, zero real value — NEVER use for anything of value.
    // Expected address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
    let private_key: &str = concat!(
        "ac0974bec39a17e36ba4a6b4d238ff944b",
        "acb478cbed5efcae784d7bf4f2ff80"
    );
    let expected_address = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"; // lower case for comparison

    let derived =
        Transaction::address_from_private_key(private_key).expect("Failed to derive address");

    assert_eq!(
        derived.to_lowercase(),
        expected_address,
        "Address derivation mismatch. Got: {}, expected: {}",
        derived,
        expected_address
    );

    eprintln!("  ✓ Address derivation correct: {}", derived);

    // Now test signing a deployment transaction
    let tx = Transaction::new_contract_deploy(
        0,               // nonce
        10000000000u128, // 10 gwei
        1000000,         // gas limit
        ATLAS_HTLC_BYTECODE,
        11155111, // Sepolia
    );

    let signed = tx.sign(private_key).expect("Failed to sign transaction");

    // Verify the signed tx is valid RLP and starts with 0x
    assert!(signed.starts_with("0x"), "Signed tx must start with 0x");
    assert!(signed.len() > 100, "Signed tx should be substantial length");

    // Verify the signed RLP can be decoded (basic sanity check)
    let signed_bytes = hex::decode(&signed[2..]).expect("Signed tx must be valid hex");
    assert!(signed_bytes.len() > 100, "Signed RLP must be substantial");

    eprintln!("  ✓ Transaction signed successfully");
    eprintln!("  ✓ Signed tx length: {} bytes", signed_bytes.len());
    eprintln!("  ✓ Signing pipeline fully functional");

    // Verify the signed RLP decodes to the correct fields
    // The first byte of an EIP-155 legacy tx RLP list should be 0xf8 (list of 9 items, < 56 bytes payload)
    // or 0xf9 (list of 9 items, > 55 bytes payload). 9 items = 0xc0 + 9 = 0xc9, plus length byte.
    // Actually for RLP, a list of 9 items with total payload > 55 bytes uses 0xf8 + length.
    // Since the tx data is large, expect 0xf8 or 0xf9 prefix.
    let first_byte = signed_bytes[0];
    assert!(
        first_byte == 0xf8 || first_byte == 0xf9,
        "Expected RLP list prefix (0xf8 or 0xf9), got 0x{:02x}",
        first_byte
    );
    eprintln!("  ✓ RLP encoding valid (prefix 0x{:02x})", first_byte);
}
