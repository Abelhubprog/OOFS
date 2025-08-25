import { useEffect, useRef, useState } from 'react';

export interface MomentsStreamEvent<T = any> {
  id?: string;
  event: string;
  data: T;
}

export function useMomentsStream(params?: { wallet?: string; kinds?: string }) {
  const [events, setEvents] = useState<MomentsStreamEvent[]>([]);
  const esRef = useRef<EventSource | null>(null);
  const backoffRef = useRef(1000);

  useEffect(() => {
    let cancelled = false;
    const qs = new URLSearchParams();
    if (params?.wallet) qs.set('channels', `moments,wallet:${params.wallet}`);
    else qs.set('channels', 'moments');
    if (params?.kinds) qs.set('kinds', params.kinds);

    const connect = () => {
      if (cancelled) return;
      const url = `/api/stream/moments?${qs.toString()}`;
      const es = new EventSource(url);
      esRef.current = es;

      es.onmessage = (e) => {
        try {
          const parsed = JSON.parse(e.data);
          setEvents((prev) => [{ event: 'message', data: parsed }, ...prev].slice(0, 100));
        } catch {
          // ignore
        }
      };

      es.onerror = () => {
        es.close();
        esRef.current = null;
        setTimeout(connect, backoffRef.current);
        backoffRef.current = Math.min(backoffRef.current * 2, 30000);
      };
    };

    connect();
    return () => {
      cancelled = true;
      esRef.current?.close();
    };
  }, [params?.wallet, params?.kinds]);

  return { events };
}

