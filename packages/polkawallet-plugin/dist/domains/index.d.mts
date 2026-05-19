import { S as SignerAccount, T as TxStatusCallback, f as ComitEvent, P as SetRecordsParams, Q as DomainInfo } from '../tx-helper-BUR0DrYk.mjs';
export { U as RegisterDomainParams, W as X3DnsRecord, Z as X3RecordData } from '../tx-helper-BUR0DrYk.mjs';
import { ApiPromise } from '@polkadot/api';
import '@polkadot/types/types';
import '@polkadot/keyring/types';

declare class DomainService {
    private api;
    constructor(api: ApiPromise);
    /** Register a .x3 domain */
    registerDomain(account: SignerAccount, domain: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Set DNS records for a domain */
    setRecords(account: SignerAccount, params: SetRecordsParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get domain info (owner + records) */
    getDomain(domain: string): Promise<DomainInfo | null>;
    /** Check if a .x3 domain is available */
    isDomainAvailable(domain: string): Promise<boolean>;
    /** List all registered domains */
    listDomains(): Promise<string[]>;
    /** Resolve a .x3 domain to its A or AAAA record */
    resolve(domain: string): Promise<string | null>;
    private _domainToBytes;
    private _encodeRecord;
    private _decodeRecord;
}

export { DomainInfo, DomainService, SetRecordsParams };
