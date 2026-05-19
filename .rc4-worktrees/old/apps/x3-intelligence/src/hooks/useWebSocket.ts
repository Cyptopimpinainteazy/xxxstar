import { useEffect, useRef } from "react";

export type MessageHandler = (msg: any) => void;

export function useWebSocket(
  url: string,
  onMessage: MessageHandler,
  onOpen?: () => void,
  onClose?: () => void
) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectRef = useRef<number>(0);

  useEffect(() => {
    let mounted = true;

    function connect() {
      if (!mounted) return;
      try {
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onopen = () => {
          reconnectRef.current = 0;
          if (onOpen) onOpen();
        };

        ws.onmessage = (ev) => {
          try {
            const data = JSON.parse(ev.data);
            onMessage(data);
          } catch (e) {
            onMessage(ev.data);
          }
        };

        ws.onclose = () => {
          if (onClose) onClose();
          // exponential backoff reconnect
          const timeout = Math.min(10000, 1000 * Math.pow(2, reconnectRef.current));
          reconnectRef.current += 1;
          setTimeout(connect, timeout);
        };

        ws.onerror = () => {
          // Close on error to trigger reconnect logic
          try {
            ws.close();
          } catch (e) {}
        };
      } catch (e) {
        // schedule reconnect
        setTimeout(connect, 1000);
      }
    }

    connect();

    return () => {
      mounted = false;
      if (wsRef.current) {
        try {
          wsRef.current.close();
        } catch (e) {}
      }
    };
  }, [url, onMessage, onOpen, onClose]);
}
