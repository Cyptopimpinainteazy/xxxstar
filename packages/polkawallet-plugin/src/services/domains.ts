/**
 * X3 Domain Registry Service — .x3 domain registration and DNS
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  RegisterDomainParams,
  SetRecordsParams,
  DomainInfo,
  X3DnsRecord,
  X3RecordData,
  TxStatusCallback,
} from '../types/interfaces';

export class DomainService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------

  /** Register a .x3 domain */
  async registerDomain(
    account: SignerAccount,
    domain: string,
    statusCb?: TxStatusCallback,
  ) {
    const domainBytes = this._domainToBytes(domain);
    const tx = this.api.tx.x3DomainRegistry.registerDomain(domainBytes);
    return signAndSend(tx, account, statusCb);
  }

  /** Set DNS records for a domain */
  async setRecords(
    account: SignerAccount,
    params: SetRecordsParams,
    statusCb?: TxStatusCallback,
  ) {
    const domainBytes = this._domainToBytes(params.domain);
    const records = params.records.map((r) => this._encodeRecord(r));
    const tx = this.api.tx.x3DomainRegistry.setRecords(domainBytes, records);
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get domain info (owner + records) */
  async getDomain(domain: string): Promise<DomainInfo | null> {
    const domainBytes = this._domainToBytes(domain);
    const info = await this.api.query.x3DomainRegistry.domains(domainBytes);
    const json = (info as any).toJSON?.();
    if (!json) return null;

    return {
      domain,
      owner: json.owner,
      records: (json.records ?? []).map(this._decodeRecord),
    };
  }

  /** Check if a .x3 domain is available */
  async isDomainAvailable(domain: string): Promise<boolean> {
    const info = await this.getDomain(domain);
    return info === null;
  }

  /** List all registered domains */
  async listDomains(): Promise<string[]> {
    const list = await this.api.query.x3DomainRegistry.domainList();
    const json = (list as any).toJSON?.() as string[][] | null;
    if (!json) return [];
    return json.map((bytes: any) =>
      Buffer.from(
        typeof bytes === 'string' ? bytes.slice(2) : bytes,
        'hex',
      ).toString(),
    );
  }

  /** Resolve a .x3 domain to its A or AAAA record */
  async resolve(domain: string): Promise<string | null> {
    const info = await this.getDomain(domain);
    if (!info) return null;

    // Find first A record
    const aRecord = info.records.find((r) => r.data.type === 'A');
    if (aRecord && aRecord.data.type === 'A') {
      return aRecord.data.value.join('.');
    }

    // Find first Txt record (often used as account address)
    const txtRecord = info.records.find((r) => r.data.type === 'Txt');
    if (txtRecord && txtRecord.data.type === 'Txt') {
      return txtRecord.data.value;
    }

    return null;
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _domainToBytes(domain: string): Uint8Array {
    // Ensure .x3 suffix
    const normalized = domain.endsWith('.x3') ? domain : `${domain}.x3`;
    return new TextEncoder().encode(normalized);
  }

  private _encodeRecord(record: X3DnsRecord) {
    let data: unknown;

    switch (record.data.type) {
      case 'A':
        data = { A: record.data.value };
        break;
      case 'Aaaa':
        data = { Aaaa: record.data.value };
        break;
      case 'Cname':
        data = { Cname: new TextEncoder().encode(record.data.value) };
        break;
      case 'Txt':
        data = { Txt: new TextEncoder().encode(record.data.value) };
        break;
    }

    return {
      ttl: record.ttl,
      data,
    };
  }

  private _decodeRecord(raw: any): X3DnsRecord {
    let data: X3RecordData;

    if (raw.data.A) {
      data = { type: 'A', value: raw.data.A };
    } else if (raw.data.Aaaa) {
      data = { type: 'Aaaa', value: raw.data.Aaaa };
    } else if (raw.data.Cname) {
      data = {
        type: 'Cname',
        value: Buffer.from(raw.data.Cname.slice(2), 'hex').toString(),
      };
    } else if (raw.data.Txt) {
      data = {
        type: 'Txt',
        value: Buffer.from(raw.data.Txt.slice(2), 'hex').toString(),
      };
    } else {
      data = { type: 'Txt', value: '' };
    }

    return { ttl: raw.ttl, data };
  }
}
