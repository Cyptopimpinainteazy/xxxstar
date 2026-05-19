/**
 * SPV Proof Generator
 * Generates SPV (Simplified Payment Verification) proofs for Bitcoin claims
 */

import crypto from 'crypto';

export interface MerkleProof {
  txHash: string;
  merkleProof: string[];
  merkleIndex: number;
}

export interface BlockHeader {
  version: string;
  prevHash: string;
  merkleRoot: string;
  time: number;
  bits: string;
  nonce: number;
}

export interface SPVProof {
  txid: string;
  blockHeight: number;
  blockHeader: BlockHeader;
  merkleProof: MerkleProof;
  confirmations: number;
  timestamp: number;
}

export interface ProofGeneratorConfig {
  rpcUrl: string;
  rpcUser?: string;
  rpcPassword?: string;
}

/**
 * Bitcoin SPV Proof Generator
 */
export class SPVProofGenerator {
  private config: ProofGeneratorConfig;

  constructor(config: ProofGeneratorConfig) {
    this.config = config;
  }

  /**
   * Generate SPV proof for transaction
   */
  async generateProof(
    txid: string,
    blockHeight: number,
    confirmations: number
  ): Promise<SPVProof> {
    // Get transaction
    const tx = await this.getTransaction(txid);

    // Get block
    const block = await this.getBlock(blockHeight);

    // Calculate merkle proof
    const merkleProof = await this.calculateMerkleProof(
      txid,
      block.tx,
      blockHeight
    );

    // Get block header
    const blockHeader = await this.getBlockHeader(blockHeight);

    return {
      txid,
      blockHeight,
      blockHeader,
      merkleProof,
      confirmations,
      timestamp: Math.floor(Date.now() / 1000),
    };
  }

  /**
   * Get transaction data
   */
  private async getTransaction(txid: string): Promise<any> {
    try {
      const tx = await this.rpcCall('gettransaction', [txid, true]);
      return tx;
    } catch (error) {
      throw new Error(`Failed to get transaction ${txid}: ${error}`);
    }
  }

  /**
   * Get block data
   */
  private async getBlock(blockHeight: number): Promise<any> {
    try {
      const blockHash = await this.rpcCall('getblockhash', [blockHeight]);
      const block = await this.rpcCall('getblock', [blockHash, 2]);
      return block;
    } catch (error) {
      throw new Error(`Failed to get block ${blockHeight}: ${error}`);
    }
  }

  /**
   * Get block header
   */
  private async getBlockHeader(blockHeight: number): Promise<BlockHeader> {
    try {
      const blockHash = await this.rpcCall('getblockhash', [blockHeight]);
      const blockInfo = await this.rpcCall('getblockheader', [blockHash]);

      return {
        version: blockInfo.version.toString(16),
        prevHash: blockInfo.previousblockhash,
        merkleRoot: blockInfo.merkleroot,
        time: blockInfo.time,
        bits: blockInfo.bits,
        nonce: blockInfo.nonce,
      };
    } catch (error) {
      throw new Error(`Failed to get block header: ${error}`);
    }
  }

  /**
   * Calculate merkle proof for transaction
   */
  private async calculateMerkleProof(
    txid: string,
    blockTxs: any[],
    blockHeight: number
  ): Promise<MerkleProof> {
    // Find transaction index
    const txHashes = blockTxs.map((tx: any) => tx.txid);
    const merkleIndex = txHashes.indexOf(txid);

    if (merkleIndex === -1) {
      throw new Error(`Transaction ${txid} not found in block`);
    }

    // Build merkle proof
    const merkleProof = this.buildMerkleProof(txHashes, merkleIndex);

    return {
      txHash: txid,
      merkleProof,
      merkleIndex,
    };
  }

  /**
   * Build merkle proof tree
   */
  private buildMerkleProof(hashes: string[], targetIndex: number): string[] {
    const proof: string[] = [];
    let tree = hashes.map((h) => h.toLowerCase());

    let index = targetIndex;
    while (tree.length > 1) {
      if (tree.length % 2 !== 0) {
        tree.push(tree[tree.length - 1]); // Duplicate last
      }

      const sibling = index % 2 === 0 ? index + 1 : index - 1;
      proof.push(tree[sibling]);

      // Hash pairs
      const nextTree = [];
      for (let i = 0; i < tree.length; i += 2) {
        const hash = this.doubleSha256(tree[i] + tree[i + 1]);
        nextTree.push(hash);
      }

      tree = nextTree;
      index = Math.floor(index / 2);
    }

    return proof;
  }

  /**
   * Double SHA-256 hash
   */
  private doubleSha256(data: string): string {
    const hash1 = crypto.createHash('sha256').update(data).digest('hex');
    const hash2 = crypto.createHash('sha256').update(hash1).digest('hex');
    return hash2;
  }

  /**
   * Verify proof (for validation)
   */
  async verifyProof(proof: SPVProof): Promise<boolean> {
    // Reconstruct merkle root from proof
    const reconstructedRoot = this.reconstructMerkleRoot(
      proof.merkleProof.txHash,
      proof.merkleProof.merkleProof,
      proof.merkleProof.merkleIndex
    );

    // Compare with block header merkle root
    return reconstructedRoot.toLowerCase() === proof.blockHeader.merkleRoot.toLowerCase();
  }

  /**
   * Reconstruct merkle root from proof
   */
  private reconstructMerkleRoot(
    txHash: string,
    merkleProof: string[],
    merkleIndex: number
  ): string {
    let hash = txHash.toLowerCase();

    for (let i = 0; i < merkleProof.length; i++) {
      const proofHash = merkleProof[i].toLowerCase();
      const isLeft = (merkleIndex >> i) % 2 === 0;

      if (isLeft) {
        hash = this.doubleSha256(hash + proofHash);
      } else {
        hash = this.doubleSha256(proofHash + hash);
      }
    }

    return hash;
  }

  /**
   * Verify proof of work
   */
  async verifyProofOfWork(proof: SPVProof): Promise<boolean> {
    // Calculate hash from block header
    const headerHex = this.encodeBlockHeader(proof.blockHeader);
    const blockHash = this.doubleSha256(headerHex);

    // Check difficulty (simplified)
    const difficulty = parseInt(proof.blockHeader.bits, 16);
    const hashValue = BigInt('0x' + blockHash);
    const diffTarget = BigInt(difficulty);

    return hashValue <= diffTarget;
  }

  /**
   * Encode block header for hashing
   */
  private encodeBlockHeader(header: BlockHeader): string {
    // Simplified encoding - in production would use proper serialization
    return (
      header.version +
      header.prevHash +
      header.merkleRoot +
      header.time.toString(16).padStart(8, '0') +
      header.bits +
      header.nonce.toString(16).padStart(8, '0')
    );
  }

  /**
   * Serialize proof for transmission
   */
  serializeProof(proof: SPVProof): string {
    return JSON.stringify({
      txid: proof.txid,
      blockHeight: proof.blockHeight,
      blockHeader: {
        version: proof.blockHeader.version,
        prevHash: proof.blockHeader.prevHash,
        merkleRoot: proof.blockHeader.merkleRoot,
        time: proof.blockHeader.time,
        bits: proof.blockHeader.bits,
        nonce: proof.blockHeader.nonce,
      },
      merkleProof: {
        txHash: proof.merkleProof.txHash,
        merkleProof: proof.merkleProof.merkleProof,
        merkleIndex: proof.merkleProof.merkleIndex,
      },
      confirmations: proof.confirmations,
      timestamp: proof.timestamp,
    });
  }

  /**
   * Deserialize proof from transmission
   */
  deserializeProof(data: string): SPVProof {
    const parsed = JSON.parse(data);
    return {
      txid: parsed.txid,
      blockHeight: parsed.blockHeight,
      blockHeader: {
        version: parsed.blockHeader.version,
        prevHash: parsed.blockHeader.prevHash,
        merkleRoot: parsed.blockHeader.merkleRoot,
        time: parsed.blockHeader.time,
        bits: parsed.blockHeader.bits,
        nonce: parsed.blockHeader.nonce,
      },
      merkleProof: {
        txHash: parsed.merkleProof.txHash,
        merkleProof: parsed.merkleProof.merkleProof,
        merkleIndex: parsed.merkleProof.merkleIndex,
      },
      confirmations: parsed.confirmations,
      timestamp: parsed.timestamp,
    };
  }

  /**
   * RPC call to Bitcoin node
   */
  private async rpcCall(method: string, params: any[]): Promise<any> {
    const body = {
      jsonrpc: '2.0',
      id: Math.random().toString(36).substring(7),
      method,
      params,
    };

    const response = await fetch(this.config.rpcUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization:
          'Basic ' +
          Buffer.from(
            `${this.config.rpcUser}:${this.config.rpcPassword}`
          ).toString('base64'),
      },
      body: JSON.stringify(body),
    });

    const result = await response.json();
    if (result.error) {
      throw new Error(`RPC Error: ${result.error.message}`);
    }

    return result.result;
  }
}

export default SPVProofGenerator;
