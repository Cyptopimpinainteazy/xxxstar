'use client';

import React, { useEffect, useState } from 'react';
import { ChainStatusBanner } from '@atlas-sphere/shared/components';

// Simple WebSocket JSON-RPC client that subscribes to newHeads
export function ChainStatusContainer() {
  const [isConnected, setIsConnected] = useState(false);
  const [blockNumber, setBlockNumber] = useState<number | null>(null);
  const [ws, setWs] = useState<WebSocket | null>(null);

  useEffect(() => {
    const url = (process.env.NEXT_PUBLIC_SUBSTRATE_WS || 'ws://localhost:9944').replace('http', 'ws');
    const socket = new WebSocket(url);
    setWs(socket);

    socket.addEventListener('open', () => {
      setIsConnected(true);
      // subscribe to new heads
      const id = Math.floor(Math.random() * 1_000_000);
      const subReq = JSON.stringify({ jsonrpc: '2.0', id, method: 'chain_subscribeNewHeads', params: [] });
      socket.send(subReq);
    });

    socket.addEventListener('message', (ev) => {
      try {
        const data = JSON.parse(ev.data.toString());
        // handle subscription notifications
        if (data && data.params && data.params.result) {
          const header = data.params.result;
          if (header && header.number) {
            // header.number might be hex string '0x123'
            const n = parseInt(String(header.number).replace(/^0x/, ''), 16);
            setBlockNumber(n);
          }
        }
      } catch (e) {
        // ignore
      }
    });

    socket.addEventListener('close', () => {
      setIsConnected(false);
    });

    socket.addEventListener('error', () => {
      setIsConnected(false);
    });

    return () => {
      try {
        socket.close();
      } catch (e) {}
    };
  }, []);

  const status = isConnected ? `#${blockNumber ?? '...'}` : 'Disconnected';
  return <ChainStatusBanner status={status} isConnected={isConnected} />;
}

export default ChainStatusContainer;
