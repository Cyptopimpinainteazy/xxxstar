/**
 * Setting service — network connection utilities
 */
import { ApiPromise } from '@polkadot/api';
import { X3_SS58_PREFIX } from '../types/x3chain-types';

export async function subscribeMessage(
  method: any,
  params: any[],
  msgChannel: string,
  transform?: (data: any) => any
) {
  return method(...params, (res: any) => {
    const data = transform ? transform(res) : res;
    (window as any).send(msgChannel, data);
  }).then((unsub: () => void) => {
    const unsubFuncName = `unsub${msgChannel}`;
    (window as any)[unsubFuncName] = unsub;
    return {};
  });
}

export async function getNetworkConst(api: ApiPromise) {
  return {
    ...api.consts,
    x3chain: {
      ss58Prefix: X3_SS58_PREFIX,
      chainId: 650000,
      blockTime: 6000,
    },
  };
}

export async function getNetworkProperties(api: ApiPromise) {
  const props = await api.rpc.system.properties();
  return {
    ...props.toJSON(),
    tokenDecimals: [18],
    tokenSymbol: ['X3'],
    ss58Format: X3_SS58_PREFIX,
    chainId: 650000,
  };
}
