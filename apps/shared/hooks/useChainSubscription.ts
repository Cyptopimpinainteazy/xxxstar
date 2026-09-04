/**
 * useChainSubscription Hook
 * 
 * Provides easy access to real-time chain data with automatic subscription management.
 * Uses the ChainProvider context for shared API connection.
 */

'use client';

import { useEffect, useState, useCallback, useRef } from 'react';

// Use any types to avoid SSR issues with @polkadot/api
type ApiPromise = any;
type VoidFn = () => void;

interface ChainSubscriptionState {
  latestBlock: number;
  finalizedBlock: number;
  timestamp: number;
  validators: string[];
  isSubscribed: boolean;
}

interface UseChainSubscriptionOptions {
  enabled?: boolean;
  onNewBlock?: (blockNumber: number, hash: string) => void;
  onFinalized?: (blockNumber: number, hash: string) => void;
}

export function useChainSubscription(
  api: ApiPromise | null,
  options: UseChainSubscriptionOptions = {}
) {
  const { enabled = true, onNewBlock, onFinalized } = options;
  
  const [state, setState] = useState<ChainSubscriptionState>({
    latestBlock: 0,
    finalizedBlock: 0,
    timestamp: 0,
    validators: [],
    isSubscribed: false,
  });
  
  const unsubscribesRef = useRef<VoidFn[]>([]);

  const subscribe = useCallback(async () => {
    if (!api || !api.isConnected || !enabled) return;

    const unsubs: VoidFn[] = [];

    try {
      // Subscribe to new heads
      const unsubNewHead = await api.rpc.chain.subscribeNewHeads(async (header: any) => {
        const blockNumber = header.number.toNumber();
        const blockHash = header.hash.toHex();
        
        // Get timestamp from block
        let timestamp = Date.now();
        try {
          const signedBlock = await api.rpc.chain.getBlock(header.hash);
          const timestampExtrinsic = signedBlock.block.extrinsics.find(
            (ex: any) => ex.method.section === 'timestamp' && ex.method.method === 'set'
          );
          if (timestampExtrinsic) {
            timestamp = Number(timestampExtrinsic.method.args[0].toString());
          }
        } catch {
          // Use current time if we can't get block timestamp
        }
        
        setState((prev) => ({
          ...prev,
          latestBlock: blockNumber,
          timestamp,
          isSubscribed: true,
        }));
        
        onNewBlock?.(blockNumber, blockHash);
      });
      unsubs.push(unsubNewHead);

      // Subscribe to finalized heads
      const unsubFinalized = await api.rpc.chain.subscribeFinalizedHeads((header: any) => {
        const blockNumber = header.number.toNumber();
        const blockHash = header.hash.toHex();
        
        setState((prev) => ({
          ...prev,
          finalizedBlock: blockNumber,
        }));
        
        onFinalized?.(blockNumber, blockHash);
      });
      unsubs.push(unsubFinalized);

      // Get initial validators
      try {
        const validators = await api.query.session?.validators?.();
        if (validators) {
          setState((prev) => ({
            ...prev,
            validators: validators.map((v: any) => v.toString()),
          }));
        }
      } catch {
        // Session pallet may not exist
      }

      unsubscribesRef.current = unsubs;
    } catch (error) {
      console.error('Failed to set up chain subscriptions:', error);
    }
  }, [api, enabled, onNewBlock, onFinalized]);

  const unsubscribe = useCallback(() => {
    unsubscribesRef.current.forEach((unsub) => {
      try {
        unsub();
      } catch {
        // Ignore cleanup errors
      }
    });
    unsubscribesRef.current = [];
    setState((prev) => ({ ...prev, isSubscribed: false }));
  }, []);

  useEffect(() => {
    subscribe();
    return () => unsubscribe();
  }, [subscribe, unsubscribe]);

  return {
    ...state,
    subscribe,
    unsubscribe,
  };
}

/**
 * useAccountSubscription Hook
 * 
 * Subscribes to account balance changes and nonce updates.
 */
interface AccountSubscriptionState {
  free: bigint;
  reserved: bigint;
  frozen: bigint;
  nonce: number;
  isSubscribed: boolean;
}

export function useAccountSubscription(
  api: ApiPromise | null,
  address: string | null,
  options: { enabled?: boolean; onBalanceChange?: (free: bigint) => void } = {}
) {
  const { enabled = true, onBalanceChange } = options;
  
  const [state, setState] = useState<AccountSubscriptionState>({
    free: BigInt(0),
    reserved: BigInt(0),
    frozen: BigInt(0),
    nonce: 0,
    isSubscribed: false,
  });
  
  const unsubRef = useRef<VoidFn | null>(null);

  useEffect(() => {
    if (!api || !api.isConnected || !address || !enabled) {
      return;
    }

    const subscribe = async () => {
      try {
        const unsub = await api.query.system.account(address, (accountInfo: any) => {
          const data = accountInfo.data;
          const free = BigInt(data.free.toString());
          const reserved = BigInt(data.reserved.toString());
          const frozen = BigInt(data.frozen?.toString() || '0');
          const nonce = accountInfo.nonce.toNumber();
          
          setState({
            free,
            reserved,
            frozen,
            nonce,
            isSubscribed: true,
          });
          
          onBalanceChange?.(free);
        });
        
        unsubRef.current = unsub;
      } catch (error) {
        console.error('Failed to subscribe to account:', error);
      }
    };

    subscribe();

    return () => {
      if (unsubRef.current) {
        unsubRef.current();
        unsubRef.current = null;
      }
      setState((prev) => ({ ...prev, isSubscribed: false }));
    };
  }, [api, address, enabled, onBalanceChange]);

  return state;
}

/**
 * useEventsSubscription Hook
 * 
 * Subscribes to runtime events, optionally filtering by section/method.
 */
interface ChainEvent {
  section: string;
  method: string;
  data: any[];
  blockNumber?: number;
  index?: number;
}

interface UseEventsSubscriptionOptions {
  enabled?: boolean;
  filter?: {
    section?: string;
    method?: string;
  };
  onEvent?: (event: ChainEvent) => void;
  maxEvents?: number;
}

export function useEventsSubscription(
  api: ApiPromise | null,
  options: UseEventsSubscriptionOptions = {}
) {
  const { enabled = true, filter, onEvent, maxEvents = 100 } = options;
  
  const [events, setEvents] = useState<ChainEvent[]>([]);
  const [isSubscribed, setIsSubscribed] = useState(false);
  const unsubRef = useRef<VoidFn | null>(null);

  useEffect(() => {
    if (!api || !api.isConnected || !enabled) {
      return;
    }

    const subscribe = async () => {
      try {
        const unsub = await api.query.system.events((records: any) => {
          const newEvents: ChainEvent[] = [];
          
          records.forEach((record: any, index: number) => {
            const { event } = record;
            const section = event.section;
            const method = event.method;
            
            // Apply filter if specified
            if (filter?.section && filter.section !== section) return;
            if (filter?.method && filter.method !== method) return;
            
            const chainEvent: ChainEvent = {
              section,
              method,
              data: event.data.map((d: any) => d.toJSON()),
              index,
            };
            
            newEvents.push(chainEvent);
            onEvent?.(chainEvent);
          });
          
          if (newEvents.length > 0) {
            setEvents((prev) => {
              const combined = [...newEvents, ...prev];
              return combined.slice(0, maxEvents);
            });
          }
          
          setIsSubscribed(true);
        });
        
        unsubRef.current = unsub;
      } catch (error) {
        console.error('Failed to subscribe to events:', error);
      }
    };

    subscribe();

    return () => {
      if (unsubRef.current) {
        unsubRef.current();
        unsubRef.current = null;
      }
      setIsSubscribed(false);
    };
  }, [api, enabled, filter?.section, filter?.method, maxEvents, onEvent]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  return {
    events,
    isSubscribed,
    clearEvents,
  };
}

export default useChainSubscription;
