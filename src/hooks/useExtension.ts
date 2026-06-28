// src/hooks/useExtension.ts
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

export const useExtension = () => {
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const check = async () => {
      try {
        const status = await invoke<boolean>('extension_connected');
        setConnected(status);
      } catch {
        setConnected(false);
      }
    };

    check();
    const interval = setInterval(check, 3000);
    return () => clearInterval(interval);
  }, []);

  return { connected };
};