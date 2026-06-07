export type BondId = string;
export type BondState = 'Locked' | 'Withdrawable' | 'Slashed';
export interface DepositReceipt {
    bondId: BondId;
    txHash?: string;
}
export interface WithdrawRequest {
    requestId: string;
    bondId: BondId;
    status: 'Pending' | 'Approved' | 'Rejected';
}
export declare class CollateralManagerClient {
    private endpoint;
    private id;
    constructor(endpoint: string);
    private rpcCall;
    depositBond(account: string, asset: string, amount: bigint): Promise<DepositReceipt>;
    requestWithdrawBond(account: string, bondId: BondId): Promise<WithdrawRequest>;
    finalizeWithdraw(requestId: string): Promise<{
        txHash: string;
    }>;
    getBondState(bondId: BondId): Promise<BondState>;
    /** Get the total collateral balance for an account+asset pair. */
    getCollateral(account: string, asset: string): Promise<{
        account: string;
        asset: string;
        locked: bigint;
        available: bigint;
    }>;
    /** Lock collateral. Thin alias around `depositBond` for the named API. */
    lockCollateral(account: string, asset: string, amount: bigint): Promise<DepositReceipt>;
    /** Begin unlocking collateral. Returns the pending withdrawal request. */
    unlockCollateral(account: string, bondId: BondId): Promise<WithdrawRequest>;
    /** Status of a specific bond / collateral position. */
    getCollateralStatus(bondId: BondId): Promise<{
        bondId: BondId;
        state: BondState;
    }>;
}
//# sourceMappingURL=collateral.d.ts.map