// src/hooks/useOllama.ts
import { useEffect, useState } from 'react';

export type OllamaStatus = 'checking' | 'online' | 'offline';

export const useOllama = (baseUrl = 'http://localhost:11434') => {
  const [status, setStatus] = useState<OllamaStatus>('checking');

  useEffect(() => {
    let ignore = false;

    const checkOllama = async () => {
      try {
        const res = await fetch(`${baseUrl}/api/tags`, { signal: AbortSignal.timeout(2000) });
        if (ignore) return;
        if (res.ok) {
          setStatus('online');
        } else {
          setStatus('offline');
        }
      } catch (err) {
        if (!ignore) setStatus('offline');
      }
    };

    checkOllama();
    // Opcional: volver a verificar cada 10 segundos
    const interval = setInterval(checkOllama, 10000);
    return () => {
      ignore = true;
      clearInterval(interval);
    };
  }, [baseUrl]);

  return { status };
};