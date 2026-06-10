import React, { createContext, useContext, useState, useEffect } from 'react';

const WSContext = createContext<WebSocket | null>(null);

export const WebSocketProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [ws, setWs] = useState<WebSocket | null>(null);

  useEffect(() => {
    const socket = new WebSocket('ws://localhost:8765');
    socket.onopen = () => {
      console.log('🌐 Connected to md_supervisor backend');
      const el = document.getElementById('connection-status');
      if (el) {
        el.textContent = '🟢 connected';
        el.style.color = '#00ff00';
      }
    };
    socket.onclose = () => {
      console.log('❌ WebSocket disconnected');
      const el = document.getElementById('connection-status');
      if (el) {
        el.textContent = '🔴 disconnected';
        el.style.color = '#ff4444';
      }
    };
    setWs(socket);
    return () => {
      socket.close();
    };
  }, []);

  return <WSContext.Provider value={ws}>{children}</WSContext.Provider>;
};

export const useWebSocket = () => useContext(WSContext);