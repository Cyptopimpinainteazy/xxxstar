// src/core/tx-helper.ts
async function signAndSend(tx, account, statusCallback) {
  return new Promise((resolve, reject) => {
    const unsubPromise = tx.signAndSend(account, (result) => {
      const status = { status: "pending" };
      if (result.status.isInBlock) {
        status.status = "inBlock";
        status.blockHash = result.status.asInBlock.toHex();
        status.txHash = result.txHash.toHex();
        statusCallback?.(status);
      }
      if (result.status.isFinalized) {
        const blockHash = result.status.asFinalized.toHex();
        const events = result.events.map((record) => ({
          type: `${record.event.section}.${record.event.method}`,
          data: record.event.data.toJSON()
        }));
        const dispatchError = result.events.find(
          ({ event }) => event.section === "system" && event.method === "ExtrinsicFailed"
        );
        if (dispatchError) {
          const errorStatus = {
            status: "error",
            blockHash,
            txHash: result.txHash.toHex(),
            error: "ExtrinsicFailed",
            events
          };
          statusCallback?.(errorStatus);
          reject(new Error(`Extrinsic failed in block ${blockHash}`));
          return;
        }
        const finalStatus = {
          status: "finalized",
          blockHash,
          txHash: result.txHash.toHex(),
          events
        };
        statusCallback?.(finalStatus);
        resolve({
          blockHash,
          blockNumber: 0,
          // populated by caller if needed
          txHash: result.txHash.toHex(),
          events
        });
      }
      if (result.isError) {
        const errorStatus = {
          status: "error",
          error: "Transaction error"
        };
        statusCallback?.(errorStatus);
        reject(new Error("Transaction error"));
      }
    });
    unsubPromise.catch((err) => {
      statusCallback?.({ status: "error", error: err.message });
      reject(err);
    });
  });
}

// src/services/domains.ts
var DomainService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Register a .x3 domain */
  async registerDomain(account, domain, statusCb) {
    const domainBytes = this._domainToBytes(domain);
    const tx = this.api.tx.x3DomainRegistry.registerDomain(domainBytes);
    return signAndSend(tx, account, statusCb);
  }
  /** Set DNS records for a domain */
  async setRecords(account, params, statusCb) {
    const domainBytes = this._domainToBytes(params.domain);
    const records = params.records.map((r) => this._encodeRecord(r));
    const tx = this.api.tx.x3DomainRegistry.setRecords(domainBytes, records);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get domain info (owner + records) */
  async getDomain(domain) {
    const domainBytes = this._domainToBytes(domain);
    const info = await this.api.query.x3DomainRegistry.domains(domainBytes);
    const json = info.toJSON?.();
    if (!json) return null;
    return {
      domain,
      owner: json.owner,
      records: (json.records ?? []).map(this._decodeRecord)
    };
  }
  /** Check if a .x3 domain is available */
  async isDomainAvailable(domain) {
    const info = await this.getDomain(domain);
    return info === null;
  }
  /** List all registered domains */
  async listDomains() {
    const list = await this.api.query.x3DomainRegistry.domainList();
    const json = list.toJSON?.();
    if (!json) return [];
    return json.map(
      (bytes) => Buffer.from(
        typeof bytes === "string" ? bytes.slice(2) : bytes,
        "hex"
      ).toString()
    );
  }
  /** Resolve a .x3 domain to its A or AAAA record */
  async resolve(domain) {
    const info = await this.getDomain(domain);
    if (!info) return null;
    const aRecord = info.records.find((r) => r.data.type === "A");
    if (aRecord && aRecord.data.type === "A") {
      return aRecord.data.value.join(".");
    }
    const txtRecord = info.records.find((r) => r.data.type === "Txt");
    if (txtRecord && txtRecord.data.type === "Txt") {
      return txtRecord.data.value;
    }
    return null;
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _domainToBytes(domain) {
    const normalized = domain.endsWith(".x3") ? domain : `${domain}.x3`;
    return new TextEncoder().encode(normalized);
  }
  _encodeRecord(record) {
    let data;
    switch (record.data.type) {
      case "A":
        data = { A: record.data.value };
        break;
      case "Aaaa":
        data = { Aaaa: record.data.value };
        break;
      case "Cname":
        data = { Cname: new TextEncoder().encode(record.data.value) };
        break;
      case "Txt":
        data = { Txt: new TextEncoder().encode(record.data.value) };
        break;
    }
    return {
      ttl: record.ttl,
      data
    };
  }
  _decodeRecord(raw) {
    let data;
    if (raw.data.A) {
      data = { type: "A", value: raw.data.A };
    } else if (raw.data.Aaaa) {
      data = { type: "Aaaa", value: raw.data.Aaaa };
    } else if (raw.data.Cname) {
      data = {
        type: "Cname",
        value: Buffer.from(raw.data.Cname.slice(2), "hex").toString()
      };
    } else if (raw.data.Txt) {
      data = {
        type: "Txt",
        value: Buffer.from(raw.data.Txt.slice(2), "hex").toString()
      };
    } else {
      data = { type: "Txt", value: "" };
    }
    return { ttl: raw.ttl, data };
  }
};

export { DomainService };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map