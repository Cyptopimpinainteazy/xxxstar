import type { ChainDescriptor } from '../types';
export declare class ChainDB {
    private chains;
    private indices;
    private rpcRotation;
    constructor();
    private buildIndices;
    private initializeRotation;
    getChain(id: string): ChainDescriptor | undefined;
    getChainByChainId(chainId: number): ChainDescriptor | undefined;
    getChainsByFamilyNetwork(family: string, network: string): ChainDescriptor[];
    searchChains(query: string): ChainDescriptor[];
    getNextRpc(chainId: string): string;
    getAllEvmChains(network?: 'mainnet' | 'testnet' | 'devnet'): ChainDescriptor[];
    getAllSvmChains(network?: 'mainnet' | 'testnet' | 'devnet'): ChainDescriptor[];
    updateHealth(endpoint: string, healthy: boolean): void;
}
export declare const chainDB: ChainDB;
//# sourceMappingURL=db.d.ts.map