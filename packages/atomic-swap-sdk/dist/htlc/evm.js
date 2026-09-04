"use strict";
/**
 * EVM HTLC Adapter — Creates and manages Hash Time-Locked Contracts on EVM chains.
 *
 * Interacts with the AtlasHTLC.sol smart contract deployed on Ethereum, Polygon,
 * BSC, Arbitrum, Optimism, Base, etc.
 *
 * ABI: createHTLC(bytes32 hashLock, address recipient, address token, uint256 amount, uint256 timelock)
 *      claimHTLC(bytes32 htlcId, bytes32 secret)
 *      refundHTLC(bytes32 htlcId)
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.EvmHTLCAdapter = void 0;
exports.createEvmHTLCAdapter = createEvmHTLCAdapter;
const base_1 = require("./base");
// ─── ABI Selectors ──────────────────────────────────────────────
const SELECTOR_CREATE = "0x4b2f336d"; // createHTLC(bytes32,address,address,uint256,uint256)
const SELECTOR_CLAIM = "0x84cc315c"; // claimHTLC(bytes32,bytes32)
const SELECTOR_REFUND = "0x7249fbb6"; // refundHTLC(bytes32)
const SELECTOR_GET = "0x905d22a5"; // getHTLC(bytes32) → (sender,recipient,token,amount,hashLock,timeLock,status)
// HTLC status enum on contract
const EVM_STATUS_MAP = {
    0: "pending",
    1: "funded",
    2: "claimed",
    3: "refunded",
    4: "expired",
};
class EvmHTLCAdapter {
    chainId;
    rpcEndpoint;
    htlcContractAddress;
    constructor(chainId, rpcEndpoint, htlcContractAddress) {
        this.chainId = chainId;
        this.rpcEndpoint = rpcEndpoint;
        this.htlcContractAddress = htlcContractAddress;
    }
    async createHTLC(params, signerKey) {
        const htlcId = this.computeHTLCId(params);
        const calldata = this.encodeCreateHTLC(params);
        const isNative = this.isNativeToken(params.tokenAddress);
        const txHash = await this.sendTransaction(this.htlcContractAddress, calldata, isNative ? params.amount : "0", signerKey);
        const now = Math.floor(Date.now() / 1000);
        return {
            id: htlcId,
            chainId: this.chainId,
            vmType: "evm",
            hashLock: params.hashLock,
            timeLock: params.timeLock,
            sender: this.addressFromKey(signerKey),
            recipient: params.recipient,
            tokenAddress: params.tokenAddress,
            amount: params.amount,
            contractAddress: params.contractAddress || this.htlcContractAddress,
            fundingTxHash: txHash,
            status: "funded",
            createdAt: now,
            updatedAt: now,
        };
    }
    async claimHTLC(params, signerKey) {
        const calldata = this.encodeClaimHTLC(params.htlcId, params.secret);
        const txHash = await this.sendTransaction(this.htlcContractAddress, calldata, "0", signerKey);
        const htlc = await this.getHTLC(params.htlcId);
        if (!htlc)
            throw new Error(`HTLC ${params.htlcId} not found`);
        return {
            ...htlc,
            secret: params.secret,
            status: "claimed",
            updatedAt: Math.floor(Date.now() / 1000),
        };
    }
    async refundHTLC(params, signerKey) {
        const calldata = this.encodeRefundHTLC(params.htlcId);
        await this.sendTransaction(this.htlcContractAddress, calldata, "0", signerKey);
        const htlc = await this.getHTLC(params.htlcId);
        if (!htlc)
            throw new Error(`HTLC ${params.htlcId} not found`);
        return {
            ...htlc,
            status: "refunded",
            updatedAt: Math.floor(Date.now() / 1000),
        };
    }
    async getHTLC(htlcId) {
        const calldata = SELECTOR_GET + this.padBytes32(htlcId).slice(2);
        const result = await this.ethCall(this.htlcContractAddress, calldata);
        if (!result || result === "0x" || result.length < 450)
            return null;
        const data = result.slice(2); // strip 0x
        const sender = "0x" + data.slice(24, 64);
        const recipient = "0x" + data.slice(88, 128);
        const token = "0x" + data.slice(152, 192);
        const amount = BigInt("0x" + data.slice(192, 256)).toString();
        const hashLock = "0x" + data.slice(256, 320);
        const timeLock = Number(BigInt("0x" + data.slice(320, 384)));
        const statusNum = Number(BigInt("0x" + data.slice(384, 448)));
        return {
            id: htlcId,
            chainId: this.chainId,
            vmType: "evm",
            hashLock,
            timeLock,
            sender,
            recipient,
            tokenAddress: token,
            amount,
            contractAddress: this.htlcContractAddress,
            status: EVM_STATUS_MAP[statusNum] || "pending",
            createdAt: 0,
            updatedAt: Math.floor(Date.now() / 1000),
        };
    }
    async isHTLCFunded(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        return htlc?.status === "funded";
    }
    async isHTLCClaimed(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        if (htlc?.status === "claimed") {
            return { claimed: true, secret: htlc.secret };
        }
        return { claimed: false };
    }
    async isHTLCExpired(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        if (!htlc)
            return false;
        const now = Math.floor(Date.now() / 1000);
        return now > htlc.timeLock;
    }
    // ─── Encoding Helpers ───────────────────────────────────────────
    encodeCreateHTLC(params) {
        return (SELECTOR_CREATE +
            this.padBytes32(params.hashLock).slice(2) +
            this.padAddress(params.recipient).slice(2) +
            this.padAddress(params.tokenAddress).slice(2) +
            this.padUint256(params.amount).slice(2) +
            this.padUint256(params.timeLock.toString()).slice(2));
    }
    encodeClaimHTLC(htlcId, secret) {
        return (SELECTOR_CLAIM +
            this.padBytes32(htlcId).slice(2) +
            this.padBytes32(secret).slice(2));
    }
    encodeRefundHTLC(htlcId) {
        return SELECTOR_REFUND + this.padBytes32(htlcId).slice(2);
    }
    computeHTLCId(params) {
        // keccak256(abi.encodePacked(hashLock, sender, recipient, token, amount, timeLock))
        // Simplified: just hash the hashLock with the contract address
        return (0, base_1.sha256FromHex)(params.hashLock + this.htlcContractAddress.slice(2));
    }
    // ─── ABI Helpers ────────────────────────────────────────────────
    padBytes32(hex) {
        const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
        return "0x" + clean.padStart(64, "0");
    }
    padAddress(addr) {
        const clean = addr.startsWith("0x") ? addr.slice(2) : addr;
        return "0x" + clean.padStart(64, "0");
    }
    padUint256(value) {
        const big = BigInt(value);
        return "0x" + big.toString(16).padStart(64, "0");
    }
    isNativeToken(addr) {
        return (addr === "0x0000000000000000000000000000000000000000" ||
            addr === "0x0" ||
            addr === "");
    }
    addressFromKey(key) {
        const clean = key.startsWith("0x") ? key : `0x${key}`;
        const addr = (0, base_1.sha256FromHex)(clean).slice(2, 42);
        return `0x${addr}`;
    }
    // ─── RPC Helpers ────────────────────────────────────────────────
    async ethCall(to, data) {
        const body = {
            jsonrpc: "2.0",
            id: 1,
            method: "eth_call",
            params: [{ to, data }, "latest"],
        };
        const res = await fetch(this.rpcEndpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        const json = await res.json();
        return json.result || "0x";
    }
    async sendTransaction(to, data, value, signerKey) {
        const ethersMod = await import("ethers");
        const provider = new ethersMod.JsonRpcProvider(this.rpcEndpoint);
        const wallet = new ethersMod.Wallet(signerKey.startsWith("0x") ? signerKey : `0x${signerKey}`, provider);
        const nonce = await provider.getTransactionCount(wallet.address, "latest");
        const feeData = await provider.getFeeData();
        const gasPrice = feeData.gasPrice ?? ethersMod.parseUnits("20", "gwei");
        const txRequest = {
            to,
            data,
            value: BigInt(value),
            gasLimit: 300000n,
            gasPrice,
            nonce,
            chainId: (await provider.getNetwork()).chainId,
        };
        const signed = await wallet.signTransaction(txRequest);
        const response = await provider.broadcastTransaction(signed);
        await response.wait();
        return response.hash;
    }
    async getTransactionCount(_signerKey) {
        const addr = this.addressFromKey(_signerKey);
        const body = {
            jsonrpc: "2.0",
            id: 1,
            method: "eth_getTransactionCount",
            params: [addr, "latest"],
        };
        const res = await fetch(this.rpcEndpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        const json = await res.json();
        return parseInt(json.result || "0x0", 16);
    }
    async getGasPrice() {
        const body = {
            jsonrpc: "2.0",
            id: 1,
            method: "eth_gasPrice",
            params: [],
        };
        const res = await fetch(this.rpcEndpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        const json = await res.json();
        return json.result || "0x0";
    }
}
exports.EvmHTLCAdapter = EvmHTLCAdapter;
/**
 * Factory function to create an EVM HTLC adapter with env var configuration.
 * Reads X3_EVM_HTLC_CONTRACT from environment.
 */
function createEvmHTLCAdapter(chainId, rpcEndpoint) {
    const contractAddress = process.env.X3_EVM_HTLC_CONTRACT;
    if (!contractAddress) {
        throw new Error("X3_EVM_HTLC_CONTRACT environment variable is required");
    }
    return new EvmHTLCAdapter(chainId, rpcEndpoint, contractAddress);
}
//# sourceMappingURL=evm.js.map