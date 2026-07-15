// src/hooks/useSettings.ts
import { useEffect, useState } from 'react';
import { load, Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';

export type AIProvider = 'ollama';

export interface AIConfig {
  provider: AIProvider;
  apiKey: string;
  model: string;
  baseUrl: string;
}

export interface Settings {
  ai: AIConfig;
}

const DEFAULT_SETTINGS: Settings = {
  ai: {
    provider: 'ollama',
    apiKey: '',
    model: 'llama3.2',
    baseUrl: 'http://localhost:11434',
  },
};

const STORE_KEY = 'settings';
let storeInstance: Store | null = null;

const getStore = async (): Promise<Store> => {
  if (!storeInstance) storeInstance = await load('settings.json');
  return storeInstance;
};

export const useSettings = () => {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    const init = async () => {
      try {
        const store = await getStore();
        const stored = await store.get<Settings>(STORE_KEY);
        const loaded = stored ?? DEFAULT_SETTINGS;
        setSettings(loaded);
        setLoading(false);
        // Sincronizar con Rust al arrancar
        await syncAiConfig(loaded.ai);
      } catch (e) {
        console.error('[Settings] Error cargando configuración al arrancar:', e);
        setLoading(false);
      }
    };
    init();
  }, []);

  const save = async (updated: Settings) => {
    setSaving(true);
    const store = await getStore();
    await store.set(STORE_KEY, updated);
    await store.save();
    setSettings(updated);
    await syncAiConfig(updated.ai);
    setSaving(false);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return { settings, loading, saving, saved, save };
};

/**
 * Envía la configuración al backend de Rust.
 * Se llama al arrancar la app y al guardar cambios.
 */
async function syncAiConfig(config: AIConfig): Promise<void> {
  try {
    const mode = config.provider === 'ollama' ? 'local' : 'cloud';
    console.log('[Settings] Sincronizando con Rust:', { mode, model: config.model, baseUrl: config.baseUrl });
    await invoke('set_ai_config', {
      mode: mode,
      model: config.model,
      baseUrl: config.baseUrl,
    });
    console.log('[Settings] Sincronización exitosa');
  } catch (e) {
    console.error('[Settings] Error sincronizando con Rust:', e);
  }
}

/**
 * Prueba de conexión con Ollama (usa el mismo endpoint que el backend)
 */
export async function callAI(
  config: AIConfig,
  systemPrompt: string,
  userMessage: string,
): Promise<string> {
  const baseUrl = config.baseUrl;
  const res = await fetch(`${baseUrl}/api/generate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: config.model,
      prompt: `${systemPrompt}\n${userMessage}`,
      stream: false,
    }),
  });
  if (!res.ok) throw new Error(`Ollama error: ${res.status}`);
  const data = await res.json();
  return data.response || '';
}