'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

var api$1 = require('@polkadot/api');
require('@polkadot/keyring');
require('@polkadot/util-crypto');
var util = require('@polkadot/util');

var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  __defProp(target, "default", { value: mod, enumerable: true }) ,
  mod
));

// node_modules/eventemitter3/index.js
var require_eventemitter3 = __commonJS({
  "node_modules/eventemitter3/index.js"(exports, module) {
    var has = Object.prototype.hasOwnProperty;
    var prefix = "~";
    function Events() {
    }
    if (Object.create) {
      Events.prototype = /* @__PURE__ */ Object.create(null);
      if (!new Events().__proto__) prefix = false;
    }
    function EE(fn, context, once) {
      this.fn = fn;
      this.context = context;
      this.once = once || false;
    }
    function addListener(emitter, event, fn, context, once) {
      if (typeof fn !== "function") {
        throw new TypeError("The listener must be a function");
      }
      var listener = new EE(fn, context || emitter, once), evt = prefix ? prefix + event : event;
      if (!emitter._events[evt]) emitter._events[evt] = listener, emitter._eventsCount++;
      else if (!emitter._events[evt].fn) emitter._events[evt].push(listener);
      else emitter._events[evt] = [emitter._events[evt], listener];
      return emitter;
    }
    function clearEvent(emitter, evt) {
      if (--emitter._eventsCount === 0) emitter._events = new Events();
      else delete emitter._events[evt];
    }
    function EventEmitter2() {
      this._events = new Events();
      this._eventsCount = 0;
    }
    EventEmitter2.prototype.eventNames = function eventNames() {
      var names = [], events, name;
      if (this._eventsCount === 0) return names;
      for (name in events = this._events) {
        if (has.call(events, name)) names.push(prefix ? name.slice(1) : name);
      }
      if (Object.getOwnPropertySymbols) {
        return names.concat(Object.getOwnPropertySymbols(events));
      }
      return names;
    };
    EventEmitter2.prototype.listeners = function listeners(event) {
      var evt = prefix ? prefix + event : event, handlers = this._events[evt];
      if (!handlers) return [];
      if (handlers.fn) return [handlers.fn];
      for (var i = 0, l = handlers.length, ee = new Array(l); i < l; i++) {
        ee[i] = handlers[i].fn;
      }
      return ee;
    };
    EventEmitter2.prototype.listenerCount = function listenerCount(event) {
      var evt = prefix ? prefix + event : event, listeners = this._events[evt];
      if (!listeners) return 0;
      if (listeners.fn) return 1;
      return listeners.length;
    };
    EventEmitter2.prototype.emit = function emit(event, a1, a2, a3, a4, a5) {
      var evt = prefix ? prefix + event : event;
      if (!this._events[evt]) return false;
      var listeners = this._events[evt], len = arguments.length, args, i;
      if (listeners.fn) {
        if (listeners.once) this.removeListener(event, listeners.fn, void 0, true);
        switch (len) {
          case 1:
            return listeners.fn.call(listeners.context), true;
          case 2:
            return listeners.fn.call(listeners.context, a1), true;
          case 3:
            return listeners.fn.call(listeners.context, a1, a2), true;
          case 4:
            return listeners.fn.call(listeners.context, a1, a2, a3), true;
          case 5:
            return listeners.fn.call(listeners.context, a1, a2, a3, a4), true;
          case 6:
            return listeners.fn.call(listeners.context, a1, a2, a3, a4, a5), true;
        }
        for (i = 1, args = new Array(len - 1); i < len; i++) {
          args[i - 1] = arguments[i];
        }
        listeners.fn.apply(listeners.context, args);
      } else {
        var length = listeners.length, j;
        for (i = 0; i < length; i++) {
          if (listeners[i].once) this.removeListener(event, listeners[i].fn, void 0, true);
          switch (len) {
            case 1:
              listeners[i].fn.call(listeners[i].context);
              break;
            case 2:
              listeners[i].fn.call(listeners[i].context, a1);
              break;
            case 3:
              listeners[i].fn.call(listeners[i].context, a1, a2);
              break;
            case 4:
              listeners[i].fn.call(listeners[i].context, a1, a2, a3);
              break;
            default:
              if (!args) for (j = 1, args = new Array(len - 1); j < len; j++) {
                args[j - 1] = arguments[j];
              }
              listeners[i].fn.apply(listeners[i].context, args);
          }
        }
      }
      return true;
    };
    EventEmitter2.prototype.on = function on(event, fn, context) {
      return addListener(this, event, fn, context, false);
    };
    EventEmitter2.prototype.once = function once(event, fn, context) {
      return addListener(this, event, fn, context, true);
    };
    EventEmitter2.prototype.removeListener = function removeListener(event, fn, context, once) {
      var evt = prefix ? prefix + event : event;
      if (!this._events[evt]) return this;
      if (!fn) {
        clearEvent(this, evt);
        return this;
      }
      var listeners = this._events[evt];
      if (listeners.fn) {
        if (listeners.fn === fn && (!once || listeners.once) && (!context || listeners.context === context)) {
          clearEvent(this, evt);
        }
      } else {
        for (var i = 0, events = [], length = listeners.length; i < length; i++) {
          if (listeners[i].fn !== fn || once && !listeners[i].once || context && listeners[i].context !== context) {
            events.push(listeners[i]);
          }
        }
        if (events.length) this._events[evt] = events.length === 1 ? events[0] : events;
        else clearEvent(this, evt);
      }
      return this;
    };
    EventEmitter2.prototype.removeAllListeners = function removeAllListeners(event) {
      var evt;
      if (event) {
        evt = prefix ? prefix + event : event;
        if (this._events[evt]) clearEvent(this, evt);
      } else {
        this._events = new Events();
        this._eventsCount = 0;
      }
      return this;
    };
    EventEmitter2.prototype.off = EventEmitter2.prototype.removeListener;
    EventEmitter2.prototype.addListener = EventEmitter2.prototype.on;
    EventEmitter2.prefixed = prefix;
    EventEmitter2.EventEmitter = EventEmitter2;
    if ("undefined" !== typeof module) {
      module.exports = EventEmitter2;
    }
  }
});

// src/types/x3chain-types.ts
var x3chainTypes = {
  /* ─── X3 Kernel ─── */
  ComitId: "H256",
  PrepareRoot: "H256",
  CanonicalBalance: "u128",
  AssetId: "u32",
  AssetMetadata: {
    name: "Vec<u8>",
    symbol: "Vec<u8>",
    decimals: "u8",
    total_supply: "u128"
  },
  ComitPayload: {
    evm_payload: "Option<Vec<u8>>",
    svm_payload: "Option<Vec<u8>>",
    x3_payload: "Option<Vec<u8>>",
    fee: "u128",
    deadline: "u64",
    metadata: "Option<Vec<u8>>"
  },
  /* ─── Atomic Trade Engine ─── */
  TradeBatchId: "u64",
  TradeStatus: {
    _enum: ["Pending", "Executing", "Settled", "Cancelled", "Failed"]
  },
  TradeBatch: {
    id: "TradeBatchId",
    creator: "AccountId",
    legs: "Vec<TradeLeg>",
    status: "TradeStatus",
    created_at: "BlockNumber"
  },
  TradeLeg: {
    asset_in: "AssetId",
    asset_out: "AssetId",
    amount_in: "u128",
    min_amount_out: "u128",
    chain_target: "ChainTarget"
  },
  ChainTarget: {
    _enum: ["Native", "Evm", "Svm", "X3"]
  },
  PriceObservation: {
    asset_id: "AssetId",
    price: "u128",
    timestamp: "u64",
    source: "Vec<u8>"
  },
  /* ─── X3 Verifier ─── */
  ExecutorId: "AccountId",
  X3JobId: "u64",
  X3JobStatus: {
    _enum: ["Submitted", "Assigned", "Completed", "Disputed", "Slashed"]
  },
  X3Job: {
    id: "X3JobId",
    submitter: "AccountId",
    executor: "Option<ExecutorId>",
    bytecode_hash: "H256",
    gas_limit: "u64",
    status: "X3JobStatus"
  },
  X3Receipt: {
    job_id: "X3JobId",
    executor: "ExecutorId",
    gas_used: "u64",
    return_data: "Vec<u8>",
    state_root: "H256",
    logs: "Vec<Vec<u8>>"
  },
  VerificationReport: {
    is_valid: "bool",
    gas_report: "Option<Vec<u8>>",
    safety_rules: "Option<Vec<u8>>"
  },
  /* ─── X3 Settlement Engine ─── */
  IntentId: "u64",
  SettlementStatus: {
    _enum: ["Created", "Escrowed", "ProofSubmitted", "Claimed", "Refunded", "Violated"]
  },
  Intent: {
    id: "IntentId",
    creator: "AccountId",
    chain_kind: "ChainKind",
    amount: "u128",
    asset: "AssetId",
    counterparty: "Option<AccountId>",
    proof_hash: "Option<H256>",
    status: "SettlementStatus",
    deadline: "BlockNumber"
  },
  ChainKind: {
    _enum: {
      Evm: "u64",
      Svm: "Null",
      X3: "Null",
      Bitcoin: "Null"
    }
  },
  BondState: {
    depositor: "AccountId",
    amount: "u128",
    locked_until: "Option<BlockNumber>"
  },
  /* ─── X3 Domain Registry ─── */
  DomainName: "Vec<u8>",
  DomainRecord: {
    owner: "AccountId",
    resolver: "Option<AccountId>",
    ttl: "u64",
    records: "Vec<DnsRecord>",
    registered_at: "BlockNumber",
    expires_at: "BlockNumber"
  },
  DnsRecord: {
    record_type: "DnsRecordType",
    value: "Vec<u8>"
  },
  DnsRecordType: {
    _enum: ["A", "AAAA", "CNAME", "TXT", "X3ADDR", "EVMADDR", "SVMADDR"]
  },
  /* ─── Governance ─── */
  ProposalId: "u64",
  ProposalStatus: {
    _enum: ["Active", "Approved", "Rejected", "Executed", "FastTracked", "KillSwitched"]
  },
  GovernanceProposal: {
    id: "ProposalId",
    proposer: "AccountId",
    call_data: "Vec<u8>",
    status: "ProposalStatus",
    votes_for: "u128",
    votes_against: "u128",
    is_ai_proposal: "bool"
  },
  Vote: {
    voter: "AccountId",
    amount: "u128",
    direction: "VoteDirection",
    conviction: "u8"
  },
  VoteDirection: {
    _enum: ["Aye", "Nay", "Abstain"]
  },
  /* ─── Evolution Core ─── */
  MutationId: "u64",
  MutationStatus: {
    _enum: ["Proposed", "Approved", "Applied", "Rejected", "RolledBack"]
  },
  Mutation: {
    id: "MutationId",
    proposer: "AccountId",
    description: "Vec<u8>",
    status: "MutationStatus",
    metrics_before: "Option<Vec<u8>>",
    metrics_after: "Option<Vec<u8>>"
  },
  AiAgentRegistration: {
    agent_id: "AccountId",
    name: "Vec<u8>",
    capabilities: "Vec<Vec<u8>>",
    operator: "AccountId"
  },
  /* ─── Agent Accounts ─── */
  AgentId: "AccountId",
  AgentPermissions: {
    can_trade: "bool",
    can_submit_comit: "bool",
    can_execute_x3: "bool",
    can_govern: "bool",
    max_spend_per_block: "u128"
  },
  AgentReputation: {
    score: "u64",
    total_operations: "u64",
    successful_operations: "u64",
    slashes: "u32"
  },
  /* ─── Swarm ─── */
  SwarmId: "u64",
  SwarmConfig: {
    id: "SwarmId",
    coordinator: "AccountId",
    agents: "Vec<AccountId>",
    strategy: "Vec<u8>"
  }
};
var x3chainRpc = {
  atlasKernel: {
    getCanonicalBalance: {
      description: "Get the canonical balance of an account for a given asset",
      params: [
        { name: "account", type: "AccountId" },
        { name: "asset_id", type: "AssetId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "u128"
    },
    getAssetMetadata: {
      description: "Get metadata for a registered asset",
      params: [
        { name: "asset_id", type: "AssetId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "AssetMetadata"
    },
    isAuthorized: {
      description: "Check if an account is authorized",
      params: [
        { name: "account", type: "AccountId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "bool"
    },
    getAuthorities: {
      description: "Get the list of authority accounts",
      params: [
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Vec<AccountId>"
    }
  },
  x3Domains: {
    getRecords: {
      description: "Get DNS records for a .x3 domain",
      params: [
        { name: "domain", type: "Vec<u8>" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<Vec<DnsRecord>>"
    },
    getDomain: {
      description: "Get full domain info for a .x3 domain",
      params: [
        { name: "domain", type: "Vec<u8>" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<DomainRecord>"
    },
    listDomains: {
      description: "List all registered .x3 domains",
      params: [
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Vec<DomainName>"
    }
  },
  atomicTradeEngine: {
    getBatchStatus: {
      description: "Get the status of a trade batch",
      params: [
        { name: "batch_id", type: "TradeBatchId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<TradeStatus>"
    },
    simulateTrade: {
      description: "Simulate execution of a trade batch without committing",
      params: [
        { name: "batch", type: "TradeBatch" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Vec<u8>"
    },
    getPrice: {
      description: "Get latest price observation for an asset",
      params: [
        { name: "asset_id", type: "AssetId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<PriceObservation>"
    }
  },
  x3Verifier: {
    getExecutorInfo: {
      description: "Get info about a registered X3 executor",
      params: [
        { name: "executor", type: "AccountId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<Vec<u8>>"
    },
    getJobStatus: {
      description: "Get the status of an X3 job",
      params: [
        { name: "job_id", type: "X3JobId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<X3JobStatus>"
    },
    getReceipt: {
      description: "Get the execution receipt of an X3 job",
      params: [
        { name: "job_id", type: "X3JobId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<X3Receipt>"
    }
  },
  evolutionCore: {
    getEvolutionStatus: {
      description: "Get current evolution engine status",
      params: [
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Vec<u8>"
    },
    getBlockMetrics: {
      description: "Get block-level metrics for evolution",
      params: [
        { name: "block", type: "BlockNumber" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Vec<u8>"
    }
  }
};
var X3_SS58_PREFIX = 42;
var X3_ENDPOINTS = {
  local: "ws://127.0.0.1:9944",
  testnet: "wss://testnet.x3chain.io:9944",
  mainnet: "wss://rpc.x3chain.io:9944"
};

// src/service/setting.ts
async function subscribeMessage(method, params, msgChannel, transform) {
  return method(...params, (res) => {
    const data = transform ? transform(res) : res;
    window.send(msgChannel, data);
  }).then((unsub) => {
    const unsubFuncName = `unsub${msgChannel}`;
    window[unsubFuncName] = unsub;
    return {};
  });
}
async function getNetworkConst(api2) {
  return {
    ...api2.consts,
    x3chain: {
      ss58Prefix: X3_SS58_PREFIX,
      chainId: 65e4,
      blockTime: 6e3
    }
  };
}
async function getNetworkProperties(api2) {
  const props = await api2.rpc.system.properties();
  return {
    ...props.toJSON(),
    tokenDecimals: [18],
    tokenSymbol: ["X3"],
    ss58Format: X3_SS58_PREFIX,
    chainId: 65e4
  };
}
function getApi() {
  return window.api;
}
async function submitCrossSwap(evmPayload, svmPayload, x3Payload, fee, deadline) {
  const api2 = getApi();
  return api2.tx.atlasKernel.submitComitV2(
    evmPayload,
    svmPayload,
    x3Payload,
    fee,
    deadline,
    "Cross-chain swap"
    // metadata
  );
}

// src/service/kernel.ts
function getApi2() {
  return window.api;
}
async function getCanonicalBalance(account, assetId) {
  const api2 = getApi2();
  return api2.rpc.atlasKernel.getCanonicalBalance(account, assetId);
}
async function getAssetMetadata(assetId) {
  const api2 = getApi2();
  return api2.rpc.atlasKernel.getAssetMetadata(assetId);
}
async function isAuthorized(account) {
  const api2 = getApi2();
  return api2.rpc.atlasKernel.isAuthorized(account);
}
async function getAuthorities() {
  const api2 = getApi2();
  return api2.rpc.atlasKernel.getAuthorities();
}
var submitComitV2 = submitCrossSwap;
function registerAsset(name, symbol, decimals, totalSupply) {
  const api2 = getApi2();
  return api2.tx.atlasKernel.registerAsset(name, symbol, decimals, totalSupply);
}
async function subscribeCanonicalBalance(account, assetId, msgChannel) {
  const api2 = getApi2();
  return api2.query.atlasKernel.canonicalBalances(account, assetId, (balance) => {
    window.send(msgChannel, {
      account,
      assetId,
      balance: balance.toString()
    });
  });
}
var kernel_default = {
  getCanonicalBalance,
  getAssetMetadata,
  isAuthorized,
  getAuthorities,
  submitComitV2,
  registerAsset,
  subscribeCanonicalBalance
};

// src/service/atomicTrade.ts
function getApi3() {
  return window.api;
}
async function getBatchStatus(batchId) {
  const api2 = getApi3();
  return api2.rpc.atomicTradeEngine.getBatchStatus(batchId);
}
async function simulateTrade(batch) {
  const api2 = getApi3();
  return api2.rpc.atomicTradeEngine.simulateTrade(batch);
}
async function getPrice(assetId) {
  const api2 = getApi3();
  return api2.rpc.atomicTradeEngine.getPrice(assetId);
}
async function getAllTradeBatches(account) {
  const api2 = getApi3();
  const entries = await api2.query.atomicTradeEngine.tradeBatches.entries();
  return entries.map(([key, val]) => ({
    id: key.args[0].toString(),
    ...val.toJSON()
  })).filter((b) => b.creator === account);
}
function createTradeBatch(legs) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.createTradeBatch(legs);
}
function executeTradeBatch(batchId) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.executeTradeBatch(batchId);
}
function cancelTradeBatch(batchId) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.cancelTradeBatch(batchId);
}
function registerAmmAdapter(adapterAddress, chainTarget) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.registerAmmAdapter(adapterAddress, chainTarget);
}
function submitPriceObservation(assetId, price, source) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.submitPriceObservation(assetId, price, source);
}
function executeTradeBatchViaComitV2(batchId) {
  const api2 = getApi3();
  return api2.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(batchId);
}
async function subscribeTradeBatch(batchId, msgChannel) {
  const api2 = getApi3();
  return api2.query.atomicTradeEngine.tradeBatches(batchId, (batch) => {
    window.send(msgChannel, {
      batchId,
      ...batch.toJSON()
    });
  });
}
var atomicTrade_default = {
  // queries
  getBatchStatus,
  simulateTrade,
  getPrice,
  getAllTradeBatches,
  // extrinsics
  createTradeBatch,
  executeTradeBatch,
  cancelTradeBatch,
  registerAmmAdapter,
  submitPriceObservation,
  executeTradeBatchViaComitV2,
  // subscriptions
  subscribeTradeBatch
};

// src/service/x3vm.ts
function getApi4() {
  return window.api;
}
async function getExecutorInfo(executor) {
  const api2 = getApi4();
  return api2.rpc.x3Verifier.getExecutorInfo(executor);
}
async function getJobStatus(jobId) {
  const api2 = getApi4();
  return api2.rpc.x3Verifier.getJobStatus(jobId);
}
async function getReceipt(jobId) {
  const api2 = getApi4();
  return api2.rpc.x3Verifier.getReceipt(jobId);
}
async function getAllJobs(account) {
  const api2 = getApi4();
  const entries = await api2.query.x3Verifier.jobs.entries();
  return entries.map(([key, val]) => ({
    id: key.args[0].toString(),
    ...val.toJSON()
  })).filter((j) => j.submitter === account || j.executor === account);
}
function registerExecutor(stake) {
  const api2 = getApi4();
  return api2.tx.x3Verifier.registerExecutor(stake);
}
function submitJob(bytecodeHash, bytecode, gasLimit, input) {
  const api2 = getApi4();
  return api2.tx.x3Verifier.submitJob(bytecodeHash, bytecode, gasLimit, input);
}
function submitReceipt(jobId, gasUsed, returnData, stateRoot, logs) {
  const api2 = getApi4();
  return api2.tx.x3Verifier.submitReceipt(jobId, gasUsed, returnData, stateRoot, logs);
}
function disputeReceipt(jobId, reason) {
  const api2 = getApi4();
  return api2.tx.x3Verifier.disputeReceipt(jobId, reason);
}
function toggleVerification(enabled) {
  const api2 = getApi4();
  return api2.tx.x3Verifier.toggleVerification(enabled);
}
async function subscribeJob(jobId, msgChannel) {
  const api2 = getApi4();
  return api2.query.x3Verifier.jobs(jobId, (job) => {
    window.send(msgChannel, {
      jobId,
      ...job.toJSON()
    });
  });
}
var x3vm_default = {
  getExecutorInfo,
  getJobStatus,
  getReceipt,
  getAllJobs,
  registerExecutor,
  submitJob,
  submitReceipt,
  disputeReceipt,
  toggleVerification,
  subscribeJob
};

// src/service/x3domains.ts
function getApi5() {
  return window.api;
}
async function getDomain(domain) {
  const api2 = getApi5();
  return api2.rpc.x3Domains.getDomain(domain);
}
async function getRecords(domain) {
  const api2 = getApi5();
  return api2.rpc.x3Domains.getRecords(domain);
}
async function listDomains() {
  const api2 = getApi5();
  return api2.rpc.x3Domains.listDomains();
}
async function getOwnedDomains(account) {
  const api2 = getApi5();
  const entries = await api2.query.x3DomainRegistry.domains.entries();
  return entries.map(([key, val]) => ({
    name: key.args[0].toHuman(),
    ...val.toJSON()
  })).filter((d) => d.owner === account);
}
async function resolve(domain) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON();
  const x3addr = recordList.find((r) => r.record_type === "X3ADDR");
  return x3addr ? x3addr.value : null;
}
async function resolveEvm(domain) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON();
  const evmAddr = recordList.find((r) => r.record_type === "EVMADDR");
  return evmAddr ? evmAddr.value : null;
}
async function resolveSvm(domain) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON();
  const svmAddr = recordList.find((r) => r.record_type === "SVMADDR");
  return svmAddr ? svmAddr.value : null;
}
function registerDomain(domain, duration) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.registerDomain(domain, duration);
}
function setRecord(domain, recordType, value) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.setRecord(domain, recordType, value);
}
function removeRecord(domain, recordType) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.removeRecord(domain, recordType);
}
function transferDomain(domain, newOwner) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.transferDomain(domain, newOwner);
}
function renewDomain(domain, additionalDuration) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.renewDomain(domain, additionalDuration);
}
function setResolver(domain, resolver) {
  const api2 = getApi5();
  return api2.tx.x3DomainRegistry.setResolver(domain, resolver);
}
var x3domains_default = {
  getDomain,
  getRecords,
  listDomains,
  getOwnedDomains,
  resolve,
  resolveEvm,
  resolveSvm,
  registerDomain,
  setRecord,
  removeRecord,
  transferDomain,
  renewDomain,
  setResolver
};

// src/service/governance.ts
function getApi6() {
  return window.api;
}
async function getProposal(proposalId) {
  const api2 = getApi6();
  return api2.query.governance.proposals(proposalId);
}
async function getAllProposals() {
  const api2 = getApi6();
  const entries = await api2.query.governance.proposals.entries();
  return entries.map(([key, val]) => ({
    id: key.args[0].toString(),
    ...val.toJSON()
  }));
}
async function getActiveProposals() {
  const all = await getAllProposals();
  return all.filter((p) => p.status === "Active");
}
async function getVotes(proposalId) {
  const api2 = getApi6();
  const entries = await api2.query.governance.votes.entries(proposalId);
  return entries.map(([key, val]) => ({
    voter: key.args[1].toHuman(),
    ...val.toJSON()
  }));
}
async function getDelegations(account) {
  const api2 = getApi6();
  return api2.query.governance.delegations(account);
}
function submitProposal(callData, description) {
  const api2 = getApi6();
  return api2.tx.governance.submitProposal(callData, description);
}
function vote(proposalId, direction, amount, conviction) {
  const api2 = getApi6();
  return api2.tx.governance.vote(proposalId, direction, amount, conviction);
}
function delegate(target, amount, conviction) {
  const api2 = getApi6();
  return api2.tx.governance.delegate(target, amount, conviction);
}
function fastTrack(proposalId) {
  const api2 = getApi6();
  return api2.tx.governance.fastTrack(proposalId);
}
function submitAiProposal(callData, description, aiModelId, confidence) {
  const api2 = getApi6();
  return api2.tx.governance.submitAiProposal(callData, description, aiModelId, confidence);
}
function activateKillSwitch(reason) {
  const api2 = getApi6();
  return api2.tx.governance.activateKillSwitch(reason);
}
async function subscribeProposal(proposalId, msgChannel) {
  const api2 = getApi6();
  return api2.query.governance.proposals(proposalId, (proposal) => {
    window.send(msgChannel, {
      proposalId,
      ...proposal.toJSON()
    });
  });
}
var governance_default = {
  getProposal,
  getAllProposals,
  getActiveProposals,
  getVotes,
  getDelegations,
  submitProposal,
  vote,
  delegate,
  fastTrack,
  submitAiProposal,
  activateKillSwitch,
  subscribeProposal
};

// src/service/evolution.ts
function getApi7() {
  return window.api;
}
async function getEvolutionStatus() {
  const api2 = getApi7();
  return api2.rpc.evolutionCore.getEvolutionStatus();
}
async function getBlockMetrics(blockNumber) {
  const api2 = getApi7();
  return api2.rpc.evolutionCore.getBlockMetrics(blockNumber);
}
async function getMutation(mutationId) {
  const api2 = getApi7();
  return api2.query.evolutionCore.mutations(mutationId);
}
async function getAllMutations() {
  const api2 = getApi7();
  const entries = await api2.query.evolutionCore.mutations.entries();
  return entries.map(([key, val]) => ({
    id: key.args[0].toString(),
    ...val.toJSON()
  }));
}
async function getActiveMutations() {
  const all = await getAllMutations();
  return all.filter((m) => m.status === "Proposed" || m.status === "Approved");
}
async function getRegisteredAgents() {
  const api2 = getApi7();
  const entries = await api2.query.evolutionCore.aiAgents.entries();
  return entries.map(([key, val]) => ({
    agentId: key.args[0].toHuman(),
    ...val.toJSON()
  }));
}
function proposeMutation(description, parameters) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.proposeMutation(description, parameters);
}
function approveMutation(mutationId) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.approveMutation(mutationId);
}
function recordMetrics(metricsData) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.recordMetrics(metricsData);
}
function registerAiAgent(name, capabilities, operator) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.registerAiAgent(name, capabilities, operator);
}
function emergencyStop(reason) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.emergencyStop(reason);
}
function rollbackMutation(mutationId, reason) {
  const api2 = getApi7();
  return api2.tx.evolutionCore.rollbackMutation(mutationId, reason);
}
var evolution_default = {
  getEvolutionStatus,
  getBlockMetrics,
  getMutation,
  getAllMutations,
  getActiveMutations,
  getRegisteredAgents,
  proposeMutation,
  approveMutation,
  recordMetrics,
  registerAiAgent,
  emergencyStop,
  rollbackMutation
};

// src/service/settlement.ts
function getApi8() {
  return window.api;
}
async function getIntent(intentId) {
  const api2 = getApi8();
  return api2.query.x3SettlementEngine.intents(intentId);
}
async function getAllIntents(account) {
  const api2 = getApi8();
  const entries = await api2.query.x3SettlementEngine.intents.entries();
  return entries.map(([key, val]) => ({
    id: key.args[0].toString(),
    ...val.toJSON()
  })).filter((i) => i.creator === account || i.counterparty === account);
}
async function getBondState(account) {
  const api2 = getApi8();
  return api2.query.x3SettlementEngine.bonds(account);
}
async function getEscrowBalance(intentId) {
  const api2 = getApi8();
  return api2.query.x3SettlementEngine.escrows(intentId);
}
function createIntent(chainKind, amount, asset, counterparty, deadline) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.createIntent(
    chainKind,
    amount,
    asset,
    counterparty,
    deadline
  );
}
function lockEscrow(intentId, amount) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.lockEscrow(intentId, amount);
}
function submitProof(intentId, proofData) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.submitProof(intentId, proofData);
}
function claimSettlement(intentId) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.claimSettlement(intentId);
}
function refundSettlement(intentId) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.refundSettlement(intentId);
}
function depositBond(amount) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.depositBond(amount);
}
function submitBtcProof(intentId, btcTxHash, merkleProof) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.submitBtcProof(intentId, btcTxHash, merkleProof);
}
function submitBtcHeader(headerData) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.submitBtcHeader(headerData);
}
function reportViolation(target, evidence) {
  const api2 = getApi8();
  return api2.tx.x3SettlementEngine.reportViolation(target, evidence);
}
async function subscribeIntent(intentId, msgChannel) {
  const api2 = getApi8();
  return api2.query.x3SettlementEngine.intents(intentId, (intent) => {
    window.send(msgChannel, {
      intentId,
      ...intent.toJSON()
    });
  });
}
var settlement_default = {
  getIntent,
  getAllIntents,
  getBondState,
  getEscrowBalance,
  createIntent,
  lockEscrow,
  submitProof,
  claimSettlement,
  refundSettlement,
  depositBond,
  submitBtcProof,
  submitBtcHeader,
  reportViolation,
  subscribeIntent
};

// src/service/agents.ts
function getApi9() {
  return window.api;
}
async function getAgent(agentId) {
  const api2 = getApi9();
  return api2.query.agentAccounts.agents(agentId);
}
async function getAgentPermissions(agentId) {
  const api2 = getApi9();
  return api2.query.agentAccounts.permissions(agentId);
}
async function getAgentReputation(agentId) {
  const api2 = getApi9();
  return api2.query.agentAccounts.reputation(agentId);
}
async function getAllAgents() {
  const api2 = getApi9();
  const entries = await api2.query.agentAccounts.agents.entries();
  return entries.map(([key, val]) => ({
    agentId: key.args[0].toHuman(),
    ...val.toJSON()
  }));
}
async function getMyAgents(operator) {
  const all = await getAllAgents();
  return all.filter((a) => a.operator === operator);
}
function registerAgent(name, permissions) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.registerAgent(name, permissions);
}
function updateOperator(agentId, newOperator) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.updateOperator(agentId, newOperator);
}
function updatePermissions(agentId, permissions) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.updatePermissions(agentId, permissions);
}
function suspendAgent(agentId, reason) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.suspendAgent(agentId, reason);
}
function recordConsumption(agentId, amount) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.recordConsumption(agentId, amount);
}
function updateReputation(agentId, delta) {
  const api2 = getApi9();
  return api2.tx.agentAccounts.updateReputation(agentId, delta);
}
var agents_default = {
  getAgent,
  getAgentPermissions,
  getAgentReputation,
  getAllAgents,
  getMyAgents,
  registerAgent,
  updateOperator,
  updatePermissions,
  suspendAgent,
  recordConsumption,
  updateReputation
};

// src/service/flashloan.ts
function getApi10() {
  return window.api;
}
function createFlashloanIntent(asset, amount, legs, repaymentAmount) {
  const api2 = getApi10();
  return api2.tx.x3SettlementEngine.createIntent(
    "X3",
    amount,
    asset,
    null,
    // self-settlement
    0
    // same-block deadline = flash
  );
}
function executeFlashloan(asset, borrowAmount, tradeLegs) {
  const api2 = getApi10();
  return api2.tx.utility.batchAll([
    api2.tx.x3SettlementEngine.createIntent("X3", borrowAmount, asset, null, 0),
    api2.tx.atomicTradeEngine.createTradeBatch(tradeLegs)
    // The runtime handles atomic settlement automatically
  ]);
}
function estimateFlashloanFee(amount, numLegs) {
  const amountBn = BigInt(amount);
  const baseFee = BigInt("1000000000000");
  const complexityFee = BigInt(numLegs) * BigInt("500000000000");
  const capitalFee = amountBn > 0n ? BigInt(Math.ceil(Math.log2(Number(amountBn)))) * BigInt("100000000000") : 0n;
  const totalFee = baseFee + complexityFee + capitalFee;
  return {
    baseFee: baseFee.toString(),
    complexityFee: complexityFee.toString(),
    capitalFee: capitalFee.toString(),
    totalFee: totalFee.toString()
  };
}
async function getFlashloanLiquidity(asset) {
  const api2 = getApi10();
  const poolAccount = api2.registry.createType(
    "AccountId",
    new Uint8Array(32).fill(0)
    // placeholder — actual pool derived at runtime
  );
  return api2.query.system.account(poolAccount);
}
var flashloan_default = {
  createFlashloanIntent,
  executeFlashloan,
  estimateFlashloanFee,
  getFlashloanLiquidity
};

// node_modules/eventemitter3/index.mjs
var import_index = __toESM(require_eventemitter3());

// src/types/runtime-types.ts
var X3ChainCustomTypes = {
  // --- x3-kernel ---
  ComitFailureReason: {
    _enum: [
      "EvmExecutionFailed",
      "SvmExecutionFailed",
      "X3ExecutionFailed",
      "InvalidNonce",
      "InsufficientBalance",
      "PayloadDecodeError"
    ]
  },
  AssetMetadata: {
    symbol: "Vec<u8>",
    decimals: "u8",
    registered_at: "BlockNumber"
  },
  AtlasId: {
    account: "AccountId",
    nonce: "u64",
    registered_block: "BlockNumber"
  },
  // --- x3-settlement-engine ---
  SettlementIntent: {
    maker: "AccountId",
    taker: "AccountId",
    asset_a: "AssetSpec",
    asset_b: "AssetSpec",
    secret_hash: "H256",
    timeout: "u64",
    created_at: "BlockNumber"
  },
  AssetSpec: {
    chain: "ExternalChainId",
    asset_id: "Vec<u8>",
    amount: "u128"
  },
  ExternalChainId: {
    _enum: [
      "X3",
      "Ethereum",
      "Solana",
      "Bitcoin",
      "Polkadot",
      "Kusama",
      "Cosmos",
      "Near",
      "Avalanche",
      "Bsc",
      "Arbitrum",
      "Optimism",
      "Base",
      "Polygon"
    ]
  },
  IntentState: {
    _enum: [
      "Created",
      "Locked",
      "ProofSubmitted",
      "Claimed",
      "Refunded",
      "Expired",
      "Disputed"
    ]
  },
  EscrowLeg: {
    chain: "ExternalChainId",
    amount: "u128",
    escrow_address: "Vec<u8>",
    locked_at: "Option<BlockNumber>",
    proof: "Option<SettlementProof>"
  },
  SettlementProof: {
    _enum: {
      SubstrateEvent: "(H256, u32)",
      EvmLog: "(H256, Vec<u8>)",
      SolanaSignature: "Vec<u8>",
      BtcMerkleProof: "(H256, Vec<H256>)",
      CosmosIbc: "Vec<u8>"
    }
  },
  BtcBlockHeader: {
    version: "u32",
    prev_block_hash: "H256",
    merkle_root: "H256",
    timestamp: "u32",
    bits: "u32",
    nonce: "u32"
  },
  BtcUtxoState: {
    txid: "H256",
    vout: "u32",
    amount_sats: "u64",
    confirmed: "bool",
    confirmations: "u32"
  },
  BondRecord: {
    owner: "AccountId",
    amount: "Balance",
    bond_type: "u8",
    locked: "bool",
    created_at: "BlockNumber",
    withdraw_requested_at: "Option<BlockNumber>",
    slashed: "bool"
  },
  FinalityConfig: {
    required_confirmations: "u32",
    finality_delay_blocks: "u32",
    max_reorg_depth: "u32"
  },
  InvariantViolationType: {
    _enum: [
      "PartialExecution",
      "DoubleSpend",
      "BalanceMismatch",
      "TimeoutViolation",
      "CrossVmReentrancy"
    ]
  },
  // --- x3-domain-registry ---
  DomainInfo: {
    owner: "AccountId",
    records: "Vec<X3DnsRecord>"
  },
  X3DnsRecord: {
    ttl: "u32",
    data: "X3RecordData"
  },
  X3RecordData: {
    _enum: {
      A: "[u8; 4]",
      Aaaa: "[u8; 16]",
      Cname: "Vec<u8>",
      Txt: "Vec<u8>"
    }
  },
  // --- x3-verifier ---
  ExecutorRecord: {
    account: "AccountId",
    stake: "Balance",
    active: "bool",
    jobs_completed: "u64",
    jobs_failed: "u64",
    registered_at: "BlockNumber"
  },
  JobRecord: {
    submitter: "AccountId",
    bytecode_hash: "H256",
    input_hash: "H256",
    gas_limit: "u128",
    reward: "Balance",
    executor: "Option<AccountId>",
    status: "JobStatus",
    created_at: "BlockNumber"
  },
  JobStatus: {
    _enum: ["Pending", "Assigned", "Completed", "Failed", "Disputed"]
  },
  ExecutionReceiptData: {
    job_id: "H256",
    executor: "AccountId",
    input_hash: "H256",
    output_hash: "H256",
    state_root_before: "H256",
    state_root_after: "H256",
    gas_used: "u128",
    timestamp: "u64",
    output_data: "Vec<u8>",
    state_changes: "Vec<(Vec<u8>, Vec<u8>)>",
    merkle_proof: "Vec<H256>",
    signature: "Vec<u8>"
  },
  // --- atomic-trade-engine ---
  VmType: {
    _enum: ["Evm", "Svm", "X3", "CrossVm"]
  },
  AmmProtocol: {
    _enum: [
      "UniswapV2",
      "UniswapV3",
      "Raydium",
      "Orca",
      "Jupiter",
      "SushiSwap",
      "PancakeSwap",
      "Curve",
      "Balancer",
      "AtlasNative"
    ]
  },
  TradeLegInput: {
    amm_protocol: "AmmProtocol",
    vm_type: "VmType",
    asset_in: "H256",
    asset_out: "H256",
    amount_in: "u128",
    min_amount_out: "u128",
    route_data: "Vec<u8>"
  },
  TradeBatch: {
    creator: "AccountId",
    legs: "Vec<TradeLegInput>",
    slippage_tolerance_bps: "u32",
    deadline: "BlockNumber",
    nonce: "u64",
    status: "TradeBatchStatus",
    created_at: "BlockNumber"
  },
  TradeBatchStatus: {
    _enum: [
      "Pending",
      "Executing",
      "Completed",
      "Failed",
      "Cancelled",
      "RolledBack"
    ]
  },
  StateCheckpoint: {
    state_root: "H256",
    block_number: "BlockNumber",
    leg_index: "u32"
  },
  AmmAdapterConfig: {
    vm_type: "VmType",
    router_address: "Vec<u8>",
    factory_address: "Vec<u8>",
    active: "bool"
  },
  PricePoint: {
    price: "u128",
    timestamp: "u64",
    source: "AmmProtocol"
  },
  TwapData: {
    cumulative_price: "u256",
    last_update: "u64",
    observation_count: "u32"
  },
  // --- governance ---
  VoteDirection: {
    _enum: ["Aye", "Nay", "Abstain"]
  },
  Conviction: {
    _enum: [
      "None",
      "Locked1x",
      "Locked2x",
      "Locked3x",
      "Locked4x",
      "Locked5x",
      "Locked6x"
    ]
  },
  ProposalStatus: {
    _enum: ["Voting", "Approved", "Rejected", "Enacted", "Cancelled"]
  },
  AIProposalType: {
    _enum: [
      "ParameterTuning",
      "FeeAdjustment",
      "SecurityPatch",
      "PerformanceOptimization",
      "ProtocolUpgrade"
    ]
  },
  KillSwitchLevel: {
    _enum: [
      "Normal",
      "Cautious",
      "Restricted",
      "UpgradeFreeze",
      "EmergencyHalt"
    ]
  },
  ImpactAssessment: {
    risk_score: "u8",
    affected_pallets: "Vec<Vec<u8>>",
    reversible: "bool",
    estimated_gas: "u128"
  },
  SimulationRequirements: {
    min_simulation_blocks: "u32",
    required_coverage_percent: "u8",
    max_state_changes: "u32"
  },
  // --- treasury ---
  SpendTrack: {
    _enum: ["SmallSpend", "MediumSpend", "BigSpend", "CriticalSpend"]
  },
  RiskLevel: {
    _enum: ["Low", "Medium", "High", "Degen"]
  },
  SpendingProposal: {
    proposer: "AccountId",
    beneficiary: "AccountId",
    amount: "Balance",
    description: "Vec<u8>",
    track: "SpendTrack",
    status: "ProposalStatus",
    created_at: "BlockNumber"
  },
  RecurringPayment: {
    beneficiary: "AccountId",
    amount: "Balance",
    interval: "BlockNumber",
    total_payments: "Option<u32>",
    payments_made: "u32",
    last_payment_at: "BlockNumber",
    active: "bool"
  },
  YieldStrategy: {
    agent: "AccountId",
    max_allocation: "Balance",
    min_expected_return: "Percent",
    risk_level: "RiskLevel",
    description: "Vec<u8>",
    active: "bool",
    total_deployed: "Balance",
    total_returned: "Balance"
  },
  // --- svm-runtime ---
  SvmAccountInfo: {
    lamports: "u64",
    owner: "[u8; 32]",
    executable: "bool",
    rent_epoch: "u64",
    data_len: "u32",
    created_at: "BlockNumber"
  },
  SvmProgramInfo: {
    upgrade_authority: "Option<[u8; 32]>",
    last_deploy_block: "BlockNumber",
    bytecode_len: "u32",
    is_frozen: "bool"
  }
};
var X3ChainRpc = {
  x3: {
    getCanonicalBalance: {
      description: "Get canonical balance for an account and asset",
      params: [
        { name: "account", type: "AccountId" },
        { name: "asset_id", type: "AssetId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Balance"
    },
    getAssetMetadata: {
      description: "Get asset metadata for a given asset_id",
      params: [
        { name: "asset_id", type: "AssetId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<AssetMetadata>"
    },
    getNonce: {
      description: "Get comit nonce for an account",
      params: [
        { name: "account", type: "AccountId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "u64"
    },
    isAuthorized: {
      description: "Check if account is authorized",
      params: [
        { name: "account", type: "AccountId" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "bool"
    },
    getAuthorities: {
      description: "Get current authority set",
      params: [{ name: "at", type: "Hash", isOptional: true }],
      type: "Vec<AccountId>"
    }
  },
  x3Settlement: {
    getIntent: {
      description: "Get settlement intent details",
      params: [
        { name: "intent_id", type: "H256" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<SettlementIntent>"
    },
    getIntentState: {
      description: "Get intent state",
      params: [
        { name: "intent_id", type: "H256" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "IntentState"
    },
    getBond: {
      description: "Get bond record",
      params: [
        { name: "bond_id", type: "H256" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<BondRecord>"
    },
    getBtcBestHeight: {
      description: "Get best known BTC block height",
      params: [{ name: "at", type: "Hash", isOptional: true }],
      type: "u64"
    }
  },
  atomicTrade: {
    getBatch: {
      description: "Get trade batch by ID",
      params: [
        { name: "batch_id", type: "H256" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<TradeBatch>"
    },
    getTwap: {
      description: "Get TWAP price for a pair",
      params: [
        { name: "token_a", type: "H256" },
        { name: "token_b", type: "H256" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<TwapData>"
    },
    getAmmAdapter: {
      description: "Get AMM adapter config",
      params: [
        { name: "protocol", type: "AmmProtocol" },
        { name: "at", type: "Hash", isOptional: true }
      ],
      type: "Option<AmmAdapterConfig>"
    }
  }
};
var X3ChainSignedExtensions = {
  ChargeTransactionPayment: {
    extrinsic: { tip: "Compact<Balance>" },
    payload: {}
  }
};

// src/config/env.ts
function getEnv(name, defaultValue) {
  if (typeof process !== "undefined" && process.env) {
    return process.env[name] || defaultValue;
  }
  return defaultValue;
}
function getEnvNumber(name, defaultValue) {
  const value = getEnv(name, defaultValue.toString()) ?? defaultValue.toString();
  const parsed = parseInt(value, 10);
  return isNaN(parsed) ? defaultValue : parsed;
}
function getEnvBoolean(name, defaultValue) {
  const value = getEnv(name, defaultValue.toString()) ?? defaultValue.toString();
  return value.toLowerCase() === "true" || value === "1";
}
var NETWORK_ENDPOINTS = {
  mainnet: getEnv("X3_RPC_ENDPOINT", "wss://rpc.x3chain.io:9944") ?? "wss://rpc.x3chain.io:9944",
  testnet: getEnv("X3_RPC_ENDPOINT", "wss://testnet.x3chain.io:9944") ?? "wss://testnet.x3chain.io:9944",
  local: getEnv("X3_RPC_ENDPOINT", "ws://127.0.0.1:9944") ?? "ws://127.0.0.1:9944"
};
function getSdkConfig() {
  const networkEnv = (getEnv("X3_NETWORK", "local") ?? "local").toLowerCase();
  const network = ["mainnet", "testnet", "local"].includes(networkEnv) ? networkEnv : "local";
  return {
    network,
    endpoint: getEnv("X3_RPC_ENDPOINT", void 0),
    autoReconnect: getEnvBoolean("X3_AUTO_RECONNECT", true),
    reconnectMaxAttempts: getEnvNumber("X3_RECONNECT_MAX", 5),
    reconnectDelay: getEnvNumber("X3_RECONNECT_DELAY", 1e3),
    timeout: getEnvNumber("X3_TIMEOUT", 3e4),
    debug: getEnvBoolean("X3_DEBUG", false)
  };
}
function getCurrentEndpoint() {
  const config = getSdkConfig();
  return config.endpoint || NETWORK_ENDPOINTS[config.network];
}

// src/core/api.ts
var X3ChainApi = class extends import_index.default {
  constructor(config = {}) {
    super();
    this._api = null;
    this._provider = null;
    this._connectionState = null;
    this._reconnectAttempts = 0;
    this._reconnectTimer = null;
    this._isDisconnecting = false;
    const envConfig = getSdkConfig();
    this._config = {
      autoConnect: true,
      timeout: 3e4,
      network: envConfig.network,
      endpoint: envConfig.endpoint,
      autoReconnect: envConfig.autoReconnect,
      reconnectMaxAttempts: envConfig.reconnectMaxAttempts,
      reconnectDelay: envConfig.reconnectDelay,
      ...config
    };
  }
  /** Get the underlying Polkadot API instance */
  get api() {
    if (!this._api) {
      throw new Error("API not connected. Call connect() first.");
    }
    return this._api;
  }
  /** Current connection state */
  get state() {
    return this._connectionState;
  }
  /** Whether the API is connected */
  get isConnected() {
    return this._api?.isConnected ?? false;
  }
  /** Get current network */
  get network() {
    return this._config.network || "local";
  }
  /**
   * Connect to the x3chain node
   */
  async connect() {
    const endpoint = this._config.endpoint || getCurrentEndpoint();
    this._isDisconnecting = false;
    this._reconnectAttempts = 0;
    this._provider = new api$1.WsProvider(endpoint, this._config.autoReconnect ? 1e3 : false);
    this._provider.on("disconnected", () => {
      if (!this._isDisconnecting) {
        this._handleDisconnect();
      } else {
        this._connectionState = null;
        this.emit("disconnected");
      }
    });
    this._provider.on("error", (err) => {
      this.emit("error", err);
    });
    try {
      this._api = await api$1.ApiPromise.create({
        provider: this._provider,
        types: X3ChainCustomTypes,
        rpc: X3ChainRpc,
        signer: this._config.signer
      });
      await this._api.isReady;
      const [chain, header] = await Promise.all([
        this._api.rpc.system.chain(),
        this._api.rpc.chain.getHeader()
      ]);
      this._connectionState = {
        connected: true,
        endpoint,
        chainName: chain.toString(),
        genesisHash: this._api.genesisHash.toHex(),
        runtimeVersion: this._api.runtimeVersion.specVersion.toNumber(),
        latestBlock: header.number.toNumber()
      };
      this.emit("connected", this._connectionState);
      this.emit("ready", this._api);
      return this._api;
    } catch (err) {
      this.emit("error", err);
      throw err;
    }
  }
  /**
   * Handle disconnection with automatic reconnection
   */
  _handleDisconnect() {
    if (this._isDisconnecting) return;
    const maxAttempts = this._config.reconnectMaxAttempts || 5;
    const baseDelay = this._config.reconnectDelay || 1e3;
    if (this._reconnectAttempts < maxAttempts) {
      this._reconnectAttempts++;
      const delay = baseDelay * Math.pow(2, this._reconnectAttempts - 1);
      this.emit("reconnecting", this._reconnectAttempts, delay);
      this._reconnectTimer = setTimeout(() => {
        this._reconnect();
      }, delay);
    } else {
      this._connectionState = null;
      this.emit("disconnected");
    }
  }
  /**
   * Attempt to reconnect to the node
   */
  async _reconnect() {
    if (this._api) {
      await this._api.disconnect();
      this._api = null;
    }
    if (this._provider) {
      await this._provider.disconnect();
      this._provider = null;
    }
    try {
      await this.connect();
      if (this._connectionState) {
        this.emit("reconnected", this._connectionState);
      }
    } catch (err) {
      this.emit("error", err);
      this._handleDisconnect();
    }
  }
  /**
   * Disconnect from the node
   */
  async disconnect() {
    this._isDisconnecting = true;
    if (this._reconnectTimer) {
      clearTimeout(this._reconnectTimer);
      this._reconnectTimer = null;
    }
    if (this._api) {
      await this._api.disconnect();
      this._api = null;
    }
    if (this._provider) {
      await this._provider.disconnect();
      this._provider = null;
    }
    this._connectionState = null;
    this.emit("disconnected");
  }
  /**
   * Set a signer (for Polkawallet mobile extension bridge)
   */
  setSigner(signer) {
    if (this._api) {
      this._api.setSigner(signer);
    }
    this._config.signer = signer;
  }
  /**
   * Get available account addresses from the connected signer/extension
   */
  async getAccounts() {
    if (!this._api) throw new Error("Not connected");
    try {
      const { web3Accounts, web3Enable } = await import('@polkadot/extension-dapp');
      await web3Enable("X3 Chain x3chain");
      const accounts = await web3Accounts();
      return accounts.map((a) => a.address);
    } catch {
      return [];
    }
  }
  /**
   * Execute a query with retry logic
   */
  async executeWithRetry(fn, maxRetries = 3, delay = 1e3) {
    let lastError;
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await fn();
      } catch (err) {
        lastError = err;
        if (attempt < maxRetries) {
          this.emit("error", new Error(`Attempt ${attempt}/${maxRetries} failed: ${lastError.message}`));
          await new Promise((resolve2) => setTimeout(resolve2, delay * attempt));
        }
      }
    }
    throw new Error(`All ${maxRetries} attempts failed: ${lastError?.message}`);
  }
  /**
   * Check if the API is connected and ready
   */
  async ensureConnected() {
    if (!this._api || !this.isConnected) {
      await this.connect();
    }
  }
};
async function createX3Api(config = {}) {
  const x3 = new X3ChainApi(config);
  await x3.connect();
  return x3;
}
async function createX3ApiFromEnv() {
  const config = getSdkConfig();
  return createX3Api({
    network: config.network,
    endpoint: config.endpoint,
    autoReconnect: config.autoReconnect,
    reconnectMaxAttempts: config.reconnectMaxAttempts,
    reconnectDelay: config.reconnectDelay
  });
}

// src/core/tx-helper.ts
async function signAndSend(tx, account, statusCallback) {
  return new Promise((resolve2, reject) => {
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
        resolve2({
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
async function estimateFee(api2, tx, account) {
  const info = await tx.paymentInfo(account);
  return info.partialFee.toBigInt();
}
var KernelService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Comit Submission
  // ---------------------------------------------------------------------------
  /**
   * Submit a Comit (dual-VM: EVM + SVM)
   */
  async submitComit(account, params, statusCb) {
    const tx = this.api.tx.atlasKernel.submitComit(
      params.comitId,
      typeof params.evmPayload === "string" ? util.hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? util.hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }
  /**
   * Submit a Comit v2 (tri-VM: EVM + SVM + X3)
   */
  async submitComitV2(account, params, statusCb) {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === "string" ? util.hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? util.hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === "string" ? util.hexToU8a(params.x3Payload) : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get canonical balance for account + asset */
  async getBalance(account, assetId) {
    const result = await this.api.query.atlasKernel.canonicalLedger(account, assetId);
    return result.toBigInt?.() ?? 0n;
  }
  /** Get all balances for an account across all registered assets */
  async getAllBalances(account) {
    const entries = await this.api.query.atlasKernel.canonicalLedger.entries(account);
    const balances = [];
    for (const [key, value] of entries) {
      const assetId = key.args[1].toNumber();
      const balance = value.toBigInt?.() ?? 0n;
      const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
      const metaJson = meta.toJSON?.();
      balances.push({
        assetId,
        symbol: metaJson?.symbol ? Buffer.from(metaJson.symbol.slice(2), "hex").toString() : `ASSET-${assetId}`,
        decimals: metaJson?.decimals ?? 18,
        free: balance,
        reserved: 0n,
        frozen: 0n
      });
    }
    return balances;
  }
  /** Get account info */
  async getAccount(address) {
    const [nonce, isAuth, systemAccount] = await Promise.all([
      this.api.query.atlasKernel.nonces(address),
      this.api.query.atlasKernel.authorizedAccounts(address),
      this.api.query.system.account(address)
    ]);
    const accountData = systemAccount.data;
    return {
      address,
      isAuthorized: isAuth.isSome ?? false,
      nonce: nonce.toBigInt?.() ?? 0n,
      freeBalance: accountData?.free?.toBigInt?.() ?? 0n,
      reservedBalance: accountData?.reserved?.toBigInt?.() ?? 0n
    };
  }
  /** Get next comit nonce for account */
  async getNonce(address) {
    const result = await this.api.query.atlasKernel.nonces(address);
    return result.toBigInt?.() ?? 0n;
  }
  /** Get asset metadata */
  async getAssetMetadata(assetId) {
    const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
    const json = meta.toJSON?.();
    if (!json) return null;
    return {
      symbol: json.symbol ? Buffer.from(json.symbol.slice(2), "hex").toString() : `ASSET-${assetId}`,
      decimals: json.decimals ?? 18
    };
  }
  /** Get the current authority set */
  async getAuthorities() {
    const result = await this.api.query.atlasKernel.authorities();
    return result.toJSON?.() ?? [];
  }
  /** Check if an account is authorized */
  async isAuthorized(address) {
    const result = await this.api.query.atlasKernel.authorizedAccounts(address);
    return result.isSome ?? false;
  }
  // ---------------------------------------------------------------------------
  // Fee estimation
  // ---------------------------------------------------------------------------
  /** Estimate fee for a comit v2 submission */
  async estimateComitFee(senderAddress, params) {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === "string" ? util.hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? util.hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === "string" ? util.hexToU8a(params.x3Payload) : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    return estimateFee(this.api, tx, senderAddress);
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _parseComitResult(comitId, result) {
    const completedEvent = result.events.find(
      (e) => e.type === "atlasKernel.ComitExecutionCompleted"
    );
    return {
      comitId,
      blockHash: result.blockHash,
      blockNumber: result.blockNumber,
      success: !result.events.some((e) => e.type === "atlasKernel.ComitFailed"),
      gasUsed: completedEvent?.data?.gas_used ? BigInt(completedEvent.data.gas_used) : void 0,
      events: result.events
    };
  }
};
var SettlementService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Create a cross-chain settlement intent (HTLC-based) */
  async createIntent(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.createIntent(
      params.taker,
      this._encodeAssetSpec(params.assetA),
      this._encodeAssetSpec(params.assetB),
      params.secretHash,
      params.timeoutSeconds ?? null
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Lock escrow for a settlement leg */
  async lockEscrow(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.lockEscrow(
      params.intentId,
      params.legIndex,
      params.chain,
      params.amount,
      typeof params.escrowData === "string" ? util.hexToU8a(params.escrowData) : params.escrowData
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit proof from an external chain */
  async submitProof(account, intentId, chain, proof, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.submitProof(intentId, chain, proof);
    return signAndSend(tx, account, statusCb);
  }
  /** Claim settlement (reveal HTLC secret) */
  async claimSettlement(account, intentId, secret, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.claimSettlement(intentId, secret);
    return signAndSend(tx, account, statusCb);
  }
  /** Refund expired settlement */
  async refundSettlement(account, intentId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.refundSettlement(intentId);
    return signAndSend(tx, account, statusCb);
  }
  /** Submit BTC transaction proof (SPV) */
  async submitBtcProof(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.submitBtcProof(
      params.intentId,
      params.btcTxid,
      params.vout,
      params.amountSats,
      params.merkleProof,
      {
        version: params.blockHeader.version,
        prev_block_hash: params.blockHeader.prevBlockHash,
        merkle_root: params.blockHeader.merkleRoot,
        timestamp: params.blockHeader.timestamp,
        bits: params.blockHeader.bits,
        nonce: params.blockHeader.nonce
      }
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit a BTC block header for the light-client */
  async submitBtcHeader(account, header, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.submitBtcHeader({
      version: header.version,
      prev_block_hash: header.prevBlockHash,
      merkle_root: header.merkleRoot,
      timestamp: header.timestamp,
      bits: header.bits,
      nonce: header.nonce
    });
    return signAndSend(tx, account, statusCb);
  }
  /** Deposit a bond */
  async depositBond(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.depositBond(
      params.asset,
      params.amount,
      params.bondType
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Request bond withdrawal */
  async requestBondWithdraw(account, bondId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.requestBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }
  /** Finalize bond withdrawal */
  async finalizeBondWithdraw(account, bondId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.finalizeBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }
  /** Report an invariant violation */
  async reportViolation(account, intentId, violationType, evidence, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.reportViolation(
      intentId,
      violationType,
      typeof evidence === "string" ? util.hexToU8a(evidence) : evidence
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get settlement intent by ID */
  async getIntent(intentId) {
    const [intent, state] = await Promise.all([
      this.api.query.x3SettlementEngine.settlementIntents(intentId),
      this.api.query.x3SettlementEngine.intentStates(intentId)
    ]);
    const json = intent.toJSON?.();
    if (!json) return null;
    return {
      intentId,
      maker: json.maker,
      taker: json.taker,
      assetA: this._decodeAssetSpec(json.asset_a),
      assetB: this._decodeAssetSpec(json.asset_b),
      secretHash: json.secret_hash,
      timeout: json.timeout,
      state: state.toString(),
      createdAt: json.created_at
    };
  }
  /** Get intent state */
  async getIntentState(intentId) {
    const state = await this.api.query.x3SettlementEngine.intentStates(intentId);
    return state.toString();
  }
  /** Get bond info */
  async getBond(bondId) {
    const bond = await this.api.query.x3SettlementEngine.bonds(bondId);
    return bond.toJSON?.() ?? null;
  }
  /** Get bonds owned by an account */
  async getBondsByOwner(account) {
    const bonds = await this.api.query.x3SettlementEngine.bondsByOwner(account);
    return bonds.toJSON?.() ?? [];
  }
  /** Get BTC best known block height */
  async getBtcBestHeight() {
    const height = await this.api.query.x3SettlementEngine.btcBestHeight();
    return height.toNumber?.() ?? 0;
  }
  /** Get protocol stats */
  async getStats() {
    const [totalIntents, totalVolume, violations] = await Promise.all([
      this.api.query.x3SettlementEngine.totalIntents(),
      this.api.query.x3SettlementEngine.totalSettledVolume(),
      this.api.query.x3SettlementEngine.invariantViolations()
    ]);
    return {
      totalIntents: totalIntents.toNumber?.() ?? 0,
      totalSettledVolume: totalVolume.toBigInt?.() ?? 0n,
      violations: violations.toNumber?.() ?? 0
    };
  }
  // ---------------------------------------------------------------------------
  // Subscriptions
  // ---------------------------------------------------------------------------
  /** Subscribe to settlement events for a given intent */
  async subscribeToIntent(intentId, callback) {
    const unsub = await this.api.query.x3SettlementEngine.intentStates(
      intentId,
      (state) => {
        callback(state.toString());
      }
    );
    return unsub;
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _encodeAssetSpec(spec) {
    return {
      chain: spec.chain,
      asset_id: spec.assetId,
      amount: spec.amount
    };
  }
  _decodeAssetSpec(raw) {
    return {
      chain: raw.chain,
      assetId: raw.asset_id,
      amount: BigInt(raw.amount)
    };
  }
};

// src/services/trades.ts
var AtomicTradeService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Create a multi-leg trade batch across EVM/SVM/X3 */
  async createTradeBatch(account, params, statusCb) {
    const legs = params.legs.map((l) => this._encodeTradeLeg(l));
    const nonce = params.nonce ?? 0n;
    const tx = this.api.tx.atomicTradeEngine.createTradeBatch(
      legs,
      params.slippageToleranceBps,
      params.deadline,
      nonce
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Execute a pending trade batch */
  async executeTradeBatch(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatch(batchId);
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }
  /** Execute trade batch through the Kernel ComitV2 path (tri-VM) */
  async executeTradeBatchViaKernel(account, batchId, comitId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(
      batchId,
      comitId
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }
  /** Cancel a pending trade batch */
  async cancelTradeBatch(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
    return signAndSend(tx, account, statusCb);
  }
  /** Create a manual checkpoint for a batch */
  async createCheckpoint(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.createManualCheckpoint(batchId);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // High-level convenience: create & execute in one shot
  // ---------------------------------------------------------------------------
  /** Create and immediately execute a trade batch */
  async trade(account, params, statusCb) {
    const createResult = await this.createTradeBatch(account, params, statusCb);
    const createdEvent = createResult.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchCreated"
    );
    const batchId = createdEvent?.data?.batch_id ?? "";
    if (!batchId) {
      throw new Error("Failed to extract batch_id from TradeBatchCreated event");
    }
    return this.executeTradeBatch(account, batchId, statusCb);
  }
  /**
   * Convenience: single-leg swap (the most common case)
   */
  async swap(account, opts, statusCb) {
    const currentBlock = (await this.api.rpc.chain.getHeader()).number.toNumber();
    return this.trade(
      account,
      {
        legs: [
          {
            ammProtocol: opts.ammProtocol,
            vmType: opts.vmType,
            assetIn: opts.assetIn,
            assetOut: opts.assetOut,
            amountIn: opts.amountIn,
            minAmountOut: opts.minAmountOut,
            routeData: opts.routeData
          }
        ],
        slippageToleranceBps: opts.slippageBps ?? 50,
        deadline: opts.deadline ?? currentBlock + 100
      },
      statusCb
    );
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get trade batch info by ID */
  async getBatch(batchId) {
    const batch = await this.api.query.atomicTradeEngine.tradeBatches(batchId);
    const json = batch.toJSON?.();
    if (!json) return null;
    return {
      batchId,
      creator: json.creator,
      legs: (json.legs ?? []).map(this._decodeTradeLeg),
      slippageToleranceBps: json.slippage_tolerance_bps,
      deadline: json.deadline,
      status: json.status,
      createdAt: json.created_at
    };
  }
  /** Get all pending batch IDs for an account */
  async getPendingBatches(account) {
    const batches = await this.api.query.atomicTradeEngine.pendingBatches(account);
    return batches.toJSON?.() ?? [];
  }
  /** Get TWAP price for a token pair */
  async getTwap(tokenA, tokenB) {
    const twap = await this.api.query.atomicTradeEngine.twapData([tokenA, tokenB]);
    const json = twap.toJSON?.();
    if (!json) return null;
    return {
      cumulativePrice: BigInt(json.cumulative_price ?? "0"),
      lastUpdate: json.last_update ?? 0
    };
  }
  /** Get AMM adapter configuration */
  async getAmmAdapter(protocol) {
    const adapter = await this.api.query.atomicTradeEngine.ammAdapters(protocol);
    return adapter.toJSON?.() ?? null;
  }
  /** Get protocol stats */
  async getStats() {
    const [completed, failed, totalVolume] = await Promise.all([
      this.api.query.atomicTradeEngine.completedBatchCount(),
      this.api.query.atomicTradeEngine.failedBatchCount(),
      this.api.query.atomicTradeEngine.totalVolume()
    ]);
    return {
      completedBatches: completed.toNumber?.() ?? 0,
      failedBatches: failed.toNumber?.() ?? 0,
      totalVolume: totalVolume.toBigInt?.() ?? 0n
    };
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _encodeTradeLeg(leg) {
    return {
      amm_protocol: leg.ammProtocol,
      vm_type: leg.vmType,
      asset_in: leg.assetIn,
      asset_out: leg.assetOut,
      amount_in: leg.amountIn,
      min_amount_out: leg.minAmountOut,
      route_data: leg.routeData ? typeof leg.routeData === "string" ? leg.routeData : Array.from(leg.routeData) : []
    };
  }
  _decodeTradeLeg(raw) {
    return {
      ammProtocol: raw.amm_protocol,
      vmType: raw.vm_type,
      assetIn: raw.asset_in,
      assetOut: raw.asset_out,
      amountIn: BigInt(raw.amount_in ?? "0"),
      minAmountOut: BigInt(raw.min_amount_out ?? "0"),
      routeData: raw.route_data
    };
  }
  _parseTradeResult(batchId, result) {
    const completedEvent = result.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchCompleted"
    );
    const failedEvent = result.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchFailed"
    );
    const legResults = result.events.filter(
      (e) => e.type === "atomicTradeEngine.TradeLegCompleted" || e.type === "atomicTradeEngine.TradeLegFailed"
    ).map((e) => ({
      legIndex: e.data.leg_index ?? 0,
      success: e.type.includes("Completed"),
      amountOut: BigInt(e.data.amount_out ?? "0"),
      error: e.data.reason
    }));
    return {
      batchId,
      success: !!completedEvent && !failedEvent,
      totalInput: BigInt(completedEvent?.data?.total_input ?? "0"),
      totalOutput: BigInt(completedEvent?.data?.total_output ?? "0"),
      gasUsed: BigInt(completedEvent?.data?.gas_used ?? "0"),
      legResults
    };
  }
};

// src/services/domains.ts
var DomainService = class {
  constructor(api2) {
    this.api = api2;
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
var VerifierService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Register as an x3vm executor (staking required) */
  async registerExecutor(account, params, statusCb) {
    const tx = this.api.tx.x3Verifier.registerExecutor(params.stake);
    return signAndSend(tx, account, statusCb);
  }
  /** Deactivate executor registration */
  async deactivateExecutor(account, statusCb) {
    const tx = this.api.tx.x3Verifier.deactivateExecutor();
    return signAndSend(tx, account, statusCb);
  }
  /** Submit a job for x3vm execution */
  async submitJob(account, params, statusCb) {
    const tx = this.api.tx.x3Verifier.submitJob(
      params.bytecodeHash,
      params.inputHash,
      params.gasLimit,
      params.reward
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit an execution receipt (proof of computation) */
  async submitReceipt(account, params, statusCb) {
    const receipt = {
      job_id: params.jobId,
      executor: typeof account === "string" ? account : account.address,
      input_hash: params.inputHash,
      output_hash: params.outputHash,
      state_root_before: params.stateRootBefore,
      state_root_after: params.stateRootAfter,
      gas_used: params.gasUsed,
      timestamp: params.timestamp,
      output_data: typeof params.outputData === "string" ? util.hexToU8a(params.outputData) : params.outputData,
      state_changes: params.stateChanges,
      merkle_proof: params.merkleProof,
      signature: typeof params.signature === "string" ? util.hexToU8a(params.signature) : params.signature
    };
    const tx = this.api.tx.x3Verifier.submitReceipt(receipt);
    return signAndSend(tx, account, statusCb);
  }
  /** Dispute a receipt */
  async disputeReceipt(account, jobId, reason, statusCb) {
    const tx = this.api.tx.x3Verifier.disputeReceipt(
      jobId,
      new TextEncoder().encode(reason)
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get job info by ID */
  async getJob(jobId) {
    const job = await this.api.query.x3Verifier.jobs(jobId);
    const json = job.toJSON?.();
    if (!json) return null;
    return {
      jobId,
      submitter: json.submitter,
      bytecodeHash: json.bytecode_hash,
      inputHash: json.input_hash,
      gasLimit: BigInt(json.gas_limit ?? "0"),
      reward: BigInt(json.reward ?? "0"),
      executor: json.executor ?? void 0,
      status: json.status,
      createdAt: json.created_at
    };
  }
  /** Get executor info */
  async getExecutor(address) {
    const exec = await this.api.query.x3Verifier.executors(address);
    return exec.toJSON?.() ?? null;
  }
  /** Get verified state root for a job */
  async getVerifiedStateRoot(jobId) {
    const root = await this.api.query.x3Verifier.verifiedStateRoots(jobId);
    const hex = root.toHex?.();
    return hex && hex !== "0x" + "00".repeat(32) ? hex : null;
  }
  /** Query if verification is globally enabled */
  async isVerificationEnabled() {
    const enabled = await this.api.query.x3Verifier.verificationEnabled();
    return enabled.isTrue ?? true;
  }
  /** Get protocol treasury balance */
  async getProtocolTreasury() {
    const treasury = await this.api.query.x3Verifier.protocolTreasury();
    return treasury.toBigInt?.() ?? 0n;
  }
  /** Get verifier stats */
  async getStats() {
    const [submitted, verified] = await Promise.all([
      this.api.query.x3Verifier.totalJobsSubmitted(),
      this.api.query.x3Verifier.totalJobsVerified()
    ]);
    return {
      totalSubmitted: submitted.toNumber?.() ?? 0,
      totalVerified: verified.toNumber?.() ?? 0
    };
  }
};

// src/services/governance.ts
var GovernanceService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Standard Governance
  // ---------------------------------------------------------------------------
  /** Submit a governance proposal */
  async submitProposal(account, params, statusCb) {
    const tx = this.api.tx.governance.submitProposal(
      params.call,
      new TextEncoder().encode(params.title),
      new TextEncoder().encode(params.description)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Vote on a proposal */
  async vote(account, params, statusCb) {
    const tx = this.api.tx.governance.vote(
      params.proposalId,
      params.direction,
      params.balance,
      params.conviction
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Delegate voting power */
  async delegate(account, params, statusCb) {
    const tx = this.api.tx.governance.delegate(params.target, params.conviction);
    return signAndSend(tx, account, statusCb);
  }
  /** Remove delegation */
  async undelegate(account, statusCb) {
    const tx = this.api.tx.governance.undelegate();
    return signAndSend(tx, account, statusCb);
  }
  /** Finalize a proposal after voting period ends */
  async finalizeProposal(account, proposalId, statusCb) {
    const tx = this.api.tx.governance.finalizeProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }
  /** Unlock tokens after conviction lock expires */
  async unlock(account, targetAccount, statusCb) {
    const tx = this.api.tx.governance.unlock(targetAccount);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // AI Governance
  // ---------------------------------------------------------------------------
  /** Submit an AI governance proposal */
  async submitAIProposal(account, params, statusCb) {
    const tx = this.api.tx.governance.submitAiProposal(
      params.proposalType,
      typeof params.payload === "string" ? new TextEncoder().encode(params.payload) : params.payload,
      {
        risk_score: params.impactAssessment.riskScore,
        affected_pallets: params.impactAssessment.affectedPallets.map(
          (p) => new TextEncoder().encode(p)
        ),
        reversible: params.impactAssessment.reversible,
        estimated_gas: params.impactAssessment.estimatedGas
      },
      {
        min_simulation_blocks: params.simulationRequirements.minSimulationBlocks,
        required_coverage_percent: params.simulationRequirements.requiredCoveragePercent,
        max_state_changes: params.simulationRequirements.maxStateChanges
      }
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Activate the kill switch (emergency) */
  async activateKillSwitch(account, level, reason, statusCb) {
    const tx = this.api.tx.governance.activateKillSwitch(
      level,
      new TextEncoder().encode(reason)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Deactivate the kill switch */
  async deactivateKillSwitch(account, statusCb) {
    const tx = this.api.tx.governance.deactivateKillSwitch();
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get proposal info */
  async getProposal(proposalId) {
    const proposal = await this.api.query.governance.proposals(proposalId);
    return proposal.toJSON?.() ?? null;
  }
  /** Get proposal tally (aye/nay/abstain counts) */
  async getProposalTally(proposalId) {
    const tally = await this.api.query.governance.proposalVotes(proposalId);
    return tally.toJSON?.() ?? null;
  }
  /** Get all active proposals */
  async getActiveProposals() {
    const count = await this.api.query.governance.proposalCount();
    const total = count.toNumber?.() ?? 0;
    const active = [];
    for (let i = 0; i < total; i++) {
      const proposal = await this.api.query.governance.proposals(i);
      const json = proposal.toJSON?.();
      if (json?.status === "Voting") {
        active.push(i);
      }
    }
    return active;
  }
  /** Get delegation info for an account */
  async getDelegation(account) {
    const delegation = await this.api.query.governance.delegations(account);
    return delegation.toJSON?.() ?? null;
  }
  /** Get current kill switch level */
  async getKillSwitchLevel() {
    const level = await this.api.query.governance.killSwitchLevelStorage();
    return level.toString();
  }
  /** Get AI proposal by ID */
  async getAIProposal(proposalId) {
    const proposal = await this.api.query.governance.aIProposals(proposalId);
    return proposal.toJSON?.() ?? null;
  }
  /** Get governance config */
  async getConfig() {
    const config = await this.api.query.governance.governanceConfig();
    return config.toJSON?.() ?? null;
  }
};

// src/services/treasury.ts
var TreasuryService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Spending Proposals
  // ---------------------------------------------------------------------------
  /** Submit a treasury spending proposal */
  async submitProposal(account, params, statusCb) {
    const tx = this.api.tx.treasury.submitProposal(
      params.beneficiary,
      params.amount,
      new TextEncoder().encode(params.description)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Approve a spending proposal (requires multi-sig signer) */
  async approveProposal(account, proposalId, statusCb) {
    const tx = this.api.tx.treasury.approveProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }
  /** Execute an approved proposal */
  async executeProposal(account, proposalId, statusCb) {
    const tx = this.api.tx.treasury.executeProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }
  /** Deposit funds into the treasury */
  async deposit(account, amount, statusCb) {
    const tx = this.api.tx.treasury.deposit(amount);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Recurring Payments
  // ---------------------------------------------------------------------------
  /** Create a recurring payment schedule */
  async createRecurringPayment(account, params, statusCb) {
    const tx = this.api.tx.treasury.createRecurringPayment(
      params.beneficiary,
      params.amount,
      params.interval,
      params.totalPayments ?? null,
      new TextEncoder().encode(params.description)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Cancel a recurring payment */
  async cancelRecurringPayment(account, paymentId, statusCb) {
    const tx = this.api.tx.treasury.cancelRecurringPayment(paymentId);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Yield Strategies
  // ---------------------------------------------------------------------------
  /** Register a yield strategy */
  async registerYieldStrategy(account, params, statusCb) {
    const tx = this.api.tx.treasury.registerYieldStrategy(
      params.agent,
      params.maxAllocation,
      params.minExpectedReturn,
      params.riskLevel,
      new TextEncoder().encode(params.description)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Execute a yield strategy (deploy capital) */
  async executeYieldStrategy(account, strategyId, amount, expectedReturn, statusCb) {
    const tx = this.api.tx.treasury.executeYieldStrategy(
      strategyId,
      amount,
      expectedReturn
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Report yield return (return capital + profit) */
  async reportYieldReturn(account, strategyId, returnedAmount, originalAmount, statusCb) {
    const tx = this.api.tx.treasury.reportYieldReturn(
      strategyId,
      returnedAmount,
      originalAmount
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Emergency Controls
  // ---------------------------------------------------------------------------
  /** Pause the treasury */
  async pause(account, reason, statusCb) {
    const tx = this.api.tx.treasury.pause(new TextEncoder().encode(reason));
    return signAndSend(tx, account, statusCb);
  }
  /** Unpause the treasury */
  async unpause(account, statusCb) {
    const tx = this.api.tx.treasury.unpause();
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get proposal info */
  async getProposal(proposalId) {
    const proposal = await this.api.query.treasury.proposals(proposalId);
    return proposal.toJSON?.() ?? null;
  }
  /** Get current signers */
  async getSigners() {
    const signers = await this.api.query.treasury.signers();
    return signers.toJSON?.() ?? [];
  }
  /** Get recurring payment info */
  async getRecurringPayment(paymentId) {
    const payment = await this.api.query.treasury.recurringPayments(paymentId);
    return payment.toJSON?.() ?? null;
  }
  /** Get yield strategy info */
  async getYieldStrategy(strategyId) {
    const strategy = await this.api.query.treasury.yieldStrategies(strategyId);
    return strategy.toJSON?.() ?? null;
  }
  /** Is the treasury paused? */
  async isPaused() {
    const paused = await this.api.query.treasury.isPaused();
    return paused.isTrue ?? false;
  }
  /** Get treasury stats */
  async getStats() {
    const stats = await this.api.query.treasury.stats();
    return stats.toJSON?.() ?? null;
  }
};

// src/services/svm.ts
var SvmService = class {
  constructor(api2) {
    this.api = api2;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Create an SVM account */
  async createAccount(account, params, statusCb) {
    const tx = this.api.tx.svmRuntime.createAccount(
      Array.from(params.pubkey),
      params.lamports,
      params.space,
      Array.from(params.owner)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Deploy an SVM program (BPF bytecode) */
  async deployProgram(account, params, statusCb) {
    const tx = this.api.tx.svmRuntime.deployProgram(
      Array.from(params.programId),
      Array.from(params.bytecode),
      params.upgradeAuthority ? Array.from(params.upgradeAuthority) : null
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Transfer lamports between SVM accounts */
  async transfer(account, params, statusCb) {
    const tx = this.api.tx.svmRuntime.transfer(
      Array.from(params.from),
      Array.from(params.to),
      params.amount
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Close an SVM account (recover lamports) */
  async closeAccount(account, pubkey, recipient, statusCb) {
    const tx = this.api.tx.svmRuntime.closeAccount(
      Array.from(pubkey),
      Array.from(recipient)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Fund an SVM account from Substrate balance */
  async fundAccount(account, svmPubkey, amount, statusCb) {
    const tx = this.api.tx.svmRuntime.fundAccount(
      Array.from(svmPubkey),
      amount
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get SVM account info */
  async getAccount(pubkey) {
    const info = await this.api.query.svmRuntime.accounts(Array.from(pubkey));
    return info.toJSON?.() ?? null;
  }
  /** Get SVM account data */
  async getAccountData(pubkey) {
    const data = await this.api.query.svmRuntime.accountData(Array.from(pubkey));
    const hex = data.toHex?.();
    if (!hex || hex === "0x") return null;
    return new Uint8Array(Buffer.from(hex.slice(2), "hex"));
  }
  /** Get SVM program info */
  async getProgram(programId) {
    const info = await this.api.query.svmRuntime.programs(Array.from(programId));
    return info.toJSON?.() ?? null;
  }
  /** Get current SVM slot */
  async getCurrentSlot() {
    const slot = await this.api.query.svmRuntime.currentSlot();
    return slot.toNumber?.() ?? 0;
  }
  /** Get total SVM lamports in system */
  async getTotalLamports() {
    const total = await this.api.query.svmRuntime.totalLamports();
    return total.toBigInt?.() ?? 0n;
  }
};
var X3VmClient = class {
  constructor(api2) {
    this.api = api2;
    this.verifier = new VerifierService(api2);
    this.kernel = new KernelService(api2);
  }
  // ---------------------------------------------------------------------------
  // Compilation (off-chain, calls x3-compiler crate via sidecar or WASM)
  // ---------------------------------------------------------------------------
  /**
   * Compile X3 Lang source code to bytecode.
   * In production this calls the x3-sidecar HTTP API or a WASM build of the compiler.
   * For now we provide the interface so Polkawallet can integrate.
   */
  async compile(source, opts) {
    const endpoint = this._getSidecarEndpoint();
    const response = await fetch(`${endpoint}/compile`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source,
        optimize: opts?.optimize ?? true,
        target: opts?.target ?? "x3vm"
      })
    });
    if (!response.ok) {
      throw new Error(`X3 compilation failed: ${response.statusText}`);
    }
    const result = await response.json();
    return {
      bytecode: new Uint8Array(result.bytecode),
      bytecodeHash: result.bytecode_hash,
      abi: result.abi,
      warnings: result.warnings ?? []
    };
  }
  // ---------------------------------------------------------------------------
  // Deployment
  // ---------------------------------------------------------------------------
  /**
   * Deploy compiled X3 bytecode to the network.
   * - Submits the bytecode as a verifier job
   * - Returns the job ID which serves as the contract address
   */
  async deploy(account, bytecode, opts, statusCb) {
    const { blake2AsHex } = await import('@polkadot/util-crypto');
    const bytecodeHash = blake2AsHex(bytecode, 256);
    const inputHash = blake2AsHex(new Uint8Array([0]), 256);
    const result = await this.verifier.submitJob(
      account,
      {
        bytecodeHash,
        inputHash,
        gasLimit: opts?.gasLimit ?? 10000000n,
        reward: opts?.reward ?? 1000000n
      },
      statusCb
    );
    const jobEvent = result.events.find(
      (e) => e.type === "x3Verifier.JobSubmitted"
    );
    const jobId = jobEvent?.data?.job_id ?? bytecodeHash;
    return {
      jobId,
      bytecodeHash,
      blockHash: result.blockHash
    };
  }
  // ---------------------------------------------------------------------------
  // Contract Calls
  // ---------------------------------------------------------------------------
  /**
   * Call an X3 contract function through the Kernel's ComitV2 path.
   * The x3Payload is ABI-encoded from the function name and args.
   */
  async call(account, params, statusCb) {
    const x3Payload = this._encodeX3Call(params);
    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);
    const senderAddress = typeof account === "string" ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);
    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: util.u8aToHex(x3Payload),
        fee: params.gasLimit ?? 1000000n,
        nonce
      },
      statusCb
    );
    return {
      success: result.success,
      output: this._decodeX3Output(result),
      gasUsed: result.gasUsed ?? 0n,
      events: result.events.filter((e) => e.type.startsWith("x3")).map((e) => ({ name: e.type, data: e.data }))
    };
  }
  /**
   * Read-only query against an X3 contract (no transaction needed).
   * Uses the x3-sidecar for dry-run execution.
   */
  async query(params) {
    const endpoint = this._getSidecarEndpoint();
    const x3Payload = this._encodeX3Call(params);
    const response = await fetch(`${endpoint}/dry-run`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        contract: params.contractAddress,
        payload: util.u8aToHex(x3Payload),
        value: (params.value ?? 0n).toString(),
        gas_limit: (params.gasLimit ?? 10000000n).toString()
      })
    });
    if (!response.ok) {
      throw new Error(`X3 query failed: ${response.statusText}`);
    }
    const result = await response.json();
    return {
      success: result.success,
      output: result.output,
      gasUsed: BigInt(result.gas_used ?? "0"),
      events: result.events ?? []
    };
  }
  // ---------------------------------------------------------------------------
  // Flash Loans (x3-flashloan integration)
  // ---------------------------------------------------------------------------
  /**
   * Execute a flash loan through the x3 flash loan pool.
   * Bundles borrow + user logic + repay into a single atomic ComitV2.
   */
  async flashLoan(account, opts, statusCb) {
    const borrowPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: "flash_borrow",
      args: [opts.asset, opts.amount.toString()]
    });
    const callbackPayload = this._encodeX3Call({
      contractAddress: opts.callbackContract,
      functionName: opts.callbackFunction,
      args: opts.callbackArgs
    });
    const repayPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: "flash_repay",
      args: [opts.asset, opts.amount.toString()]
    });
    const combinedPayload = this._encodeMultiCall([
      borrowPayload,
      callbackPayload,
      repayPayload
    ]);
    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);
    const senderAddress = typeof account === "string" ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);
    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: util.u8aToHex(combinedPayload),
        fee: 5000000n,
        nonce
      },
      statusCb
    );
    return {
      success: result.success,
      output: null,
      gasUsed: result.gasUsed ?? 0n,
      events: result.events.filter((e) => e.type.startsWith("x3") || e.type.includes("Flash")).map((e) => ({ name: e.type, data: e.data }))
    };
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _getSidecarEndpoint() {
    return "http://127.0.0.1:8080/x3";
  }
  _encodeX3Call(params) {
    const encoder = new TextEncoder();
    const contract = this._hexToBytes(params.contractAddress);
    const selector = this._functionSelector(params.functionName);
    const argsEncoded = encoder.encode(JSON.stringify(params.args));
    const payload = new Uint8Array(
      contract.length + selector.length + argsEncoded.length
    );
    payload.set(contract, 0);
    payload.set(selector, contract.length);
    payload.set(argsEncoded, contract.length + selector.length);
    return payload;
  }
  _encodeMultiCall(calls) {
    let totalLen = 4;
    for (const call of calls) totalLen += 4 + call.length;
    const buf = new Uint8Array(totalLen);
    const view = new DataView(buf.buffer);
    view.setUint32(0, calls.length, true);
    let offset = 4;
    for (const call of calls) {
      view.setUint32(offset, call.length, true);
      buf.set(call, offset + 4);
      offset += 4 + call.length;
    }
    return buf;
  }
  _functionSelector(name) {
    const encoder = new TextEncoder();
    const encoded = encoder.encode(name);
    const hash = new Uint8Array(4);
    for (let i = 0; i < encoded.length; i++) {
      hash[i % 4] ^= encoded[i];
    }
    return hash;
  }
  _hexToBytes(hex) {
    const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
    const bytes = new Uint8Array(clean.length / 2);
    for (let i = 0; i < clean.length; i += 2) {
      bytes[i / 2] = parseInt(clean.substring(i, i + 2), 16);
    }
    return bytes;
  }
  _decodeX3Output(result) {
    const outputEvent = result.events.find(
      (e) => e.type === "x3Verifier.ReceiptVerified" || e.type === "atlasKernel.ComitExecutionCompleted"
    );
    return outputEvent?.data ?? null;
  }
};

// src/plugin.ts
var AtlasX3Plugin = class {
  constructor(config) {
    this._initialized = false;
    this._x3Api = new X3ChainApi(config);
  }
  // ===========================================================================
  // Lifecycle
  // ===========================================================================
  /** Connect to the x3chain node and initialize all services */
  async init() {
    if (this._initialized) return;
    const api2 = await this._x3Api.connect();
    this._initServices(api2);
    this._initialized = true;
  }
  /** Disconnect and clean up */
  async dispose() {
    await this._x3Api.disconnect();
    this._initialized = false;
    this._kernel = void 0;
    this._settlement = void 0;
    this._trades = void 0;
    this._domains = void 0;
    this._verifier = void 0;
    this._governance = void 0;
    this._treasury = void 0;
    this._svm = void 0;
    this._x3vm = void 0;
  }
  /** Set signer for Polkawallet mobile integration */
  setSigner(signer) {
    this._x3Api.setSigner(signer);
  }
  /** Get connection state */
  get connectionState() {
    return this._x3Api.state;
  }
  /** Whether the plugin is initialized and connected */
  get isReady() {
    return this._initialized && this._x3Api.isConnected;
  }
  /** The raw Polkadot API instance (for advanced use) */
  get rawApi() {
    return this._x3Api.api;
  }
  // ===========================================================================
  // Service Accessors
  // ===========================================================================
  /** X3 Kernel — Comit submission, balances, account management */
  get kernel() {
    this._ensureReady();
    return this._kernel;
  }
  /** X3 Settlement Engine — cross-chain atomic settlement, BTC proofs, bonds */
  get settlement() {
    this._ensureReady();
    return this._settlement;
  }
  /** Atomic Trade Engine — multi-leg cross-VM trade batches, AMM routing, TWAP */
  get trades() {
    this._ensureReady();
    return this._trades;
  }
  /** X3 Domain Registry — .x3 domain registration and DNS */
  get domains() {
    this._ensureReady();
    return this._domains;
  }
  /** X3 Verifier — executor registration, job verification, state root proofs */
  get verifier() {
    this._ensureReady();
    return this._verifier;
  }
  /** Governance — proposals, voting, delegation, AI governance, kill switch */
  get governance() {
    this._ensureReady();
    return this._governance;
  }
  /** Treasury — multi-sig spending, recurring payments, yield strategies */
  get treasury() {
    this._ensureReady();
    return this._treasury;
  }
  /** SVM Runtime — Solana VM accounts, programs, transfers */
  get svm() {
    this._ensureReady();
    return this._svm;
  }
  /** X3VM — compile x3 lang, deploy contracts, call functions, flash loans */
  get x3vm() {
    this._ensureReady();
    return this._x3vm;
  }
  // ===========================================================================
  // Event subscriptions (delegated to X3ChainApi)
  // ===========================================================================
  on(event, handler) {
    this._x3Api.on(event, handler);
    return this;
  }
  off(event, handler) {
    this._x3Api.off(event, handler);
    return this;
  }
  // ===========================================================================
  // Private
  // ===========================================================================
  _initServices(api2) {
    this._kernel = new KernelService(api2);
    this._settlement = new SettlementService(api2);
    this._trades = new AtomicTradeService(api2);
    this._domains = new DomainService(api2);
    this._verifier = new VerifierService(api2);
    this._governance = new GovernanceService(api2);
    this._treasury = new TreasuryService(api2);
    this._svm = new SvmService(api2);
    this._x3vm = new X3VmClient(api2);
  }
  _ensureReady() {
    if (!this._initialized) {
      throw new Error(
        "AtlasX3Plugin not initialized. Call plugin.init() first."
      );
    }
  }
};
function createLocalPlugin() {
  return new AtlasX3Plugin({ endpoint: "ws://127.0.0.1:9944", network: "local" });
}
function createTestnetPlugin() {
  return new AtlasX3Plugin({ endpoint: getCurrentEndpoint(), network: "testnet" });
}
function createMainnetPlugin() {
  return new AtlasX3Plugin({ endpoint: "wss://rpc.x3-chain.io", network: "mainnet" });
}

// src/index.ts
function send(path, data) {
  if (typeof window !== "undefined" && window.location.href === "about:blank") {
    window.PolkaWallet?.postMessage(JSON.stringify({ path, data }));
  } else {
    console.log(`[x3chain] ${path}`, data);
  }
}
if (typeof window !== "undefined") {
  send("log", "x3chain js_api loaded");
  window.send = send;
}
var api;
async function connect(nodes) {
  return new Promise(async (resolve2, reject) => {
    const wsProvider = new api$1.WsProvider(nodes);
    try {
      api = await api$1.ApiPromise.create({
        provider: wsProvider,
        types: x3chainTypes,
        rpc: x3chainRpc
      });
      window.api = api;
      send("log", `x3chain connected: ${nodes[0]}`);
      resolve2(nodes[0]);
    } catch (err) {
      send("log", `x3chain connect failed: ${err.message}`);
      wsProvider.disconnect();
      resolve2(null);
    }
  });
}
async function connectLocal() {
  return connect([X3_ENDPOINTS.local]);
}
async function connectTestnet() {
  return connect([X3_ENDPOINTS.testnet]);
}
async function connectMainnet() {
  return connect([X3_ENDPOINTS.mainnet]);
}
async function disconnect() {
  if (api) {
    await api.disconnect();
    send("log", "x3chain disconnected");
  }
}
var settings = {
  connect,
  connectLocal,
  connectTestnet,
  connectMainnet,
  disconnect,
  subscribeMessage,
  getNetworkConst,
  getNetworkProperties
};
if (typeof window !== "undefined") {
  window.settings = settings;
  window.x3chain = {
    ...kernel_default,
    ...atomicTrade_default,
    ...x3vm_default,
    ...x3domains_default,
    ...governance_default,
    ...evolution_default,
    ...settlement_default,
    ...agents_default,
    ...flashloan_default
  };
  window.kernel = kernel_default;
  window.atomicTrade = atomicTrade_default;
  window.x3vm = x3vm_default;
  window.x3domains = x3domains_default;
  window.governance = governance_default;
  window.evolution = evolution_default;
  window.settlement = settlement_default;
  window.agents = agents_default;
  window.flashloan = flashloan_default;
}
var src_default = settings;

exports.AtlasX3Plugin = AtlasX3Plugin;
exports.AtomicTradeService = AtomicTradeService;
exports.DomainService = DomainService;
exports.GovernanceService = GovernanceService;
exports.KernelService = KernelService;
exports.SettlementService = SettlementService;
exports.SvmService = SvmService;
exports.TreasuryService = TreasuryService;
exports.VerifierService = VerifierService;
exports.X3ChainCustomTypes = X3ChainCustomTypes;
exports.X3ChainRpc = X3ChainRpc;
exports.X3ChainSignedExtensions = X3ChainSignedExtensions;
exports.X3VmClient = X3VmClient;
exports.createLocalPlugin = createLocalPlugin;
exports.createMainnetPlugin = createMainnetPlugin;
exports.createTestnetPlugin = createTestnetPlugin;
exports.createX3Api = createX3Api;
exports.createX3ApiFromEnv = createX3ApiFromEnv;
exports.default = src_default;
//# sourceMappingURL=index.js.map
//# sourceMappingURL=index.js.map