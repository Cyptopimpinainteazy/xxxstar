/**
 * x3chain routing and token configurations for Polkawallet Bridge
 */
export interface X3ChainToken {
    name: string;
    symbol: string;
    decimals: number;
    ed: string;
}
/**
 * x3chain chain configuration for the Polkawallet bridge.
 */
export declare const x3chainChainConfig: {
    id: "x3chain";
    display: string;
    type: "substrate";
    icon: string;
    paraChainId: number;
    ss58Prefix: number;
};
/**
 * Tokens available on x3chain.
 */
export declare const x3chainTokensConfig: Record<string, X3ChainToken>;
/**
 * Route configurations — defines which tokens can travel where via XCM.
 */
export declare const x3chainRouteConfigs: ({
    from: string;
    to: string;
    token: string;
    xcm: {
        fee: {
            token: string;
            amount: string;
        };
        weightLimit: string;
    };
} | {
    from: string;
    to: string;
    token: string;
    xcm: {
        fee: {
            token: string;
            amount: string;
        };
        weightLimit?: undefined;
    };
})[];
//# sourceMappingURL=configs.d.ts.map