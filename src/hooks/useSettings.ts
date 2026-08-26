// src/hooks/useSettings.ts
import { useEffect, useState } from 'react';
import { load, Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';

export type AIProvider = 'ollama' | 'spectreqa_cloud';

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
    const mode = config.provider === 'spectreqa_cloud' ? 'cloud' : 'local';
    console.log('[Settings] Sincronizando con Rust:', { mode, model: config.model, baseUrl: config.baseUrl });
    await invoke('set_ai_config', {
      mode: mode,
      model: config.model,
      baseUrl: config.baseUrl,
      apiKey: config.apiKey, // antes no se mandaba — set_ai_config lo requiere para modo cloud
    });
    console.log('[Settings] Sincronización exitosa');
  } catch (e) {
    console.error('[Settings] Error sincronizando con Rust:', e);
  }
}

/**
 * Prueba de conexión. Ollama: llamada real de generación (mismo endpoint
 * que usa Rust). Cloud: valida la key de verdad contra /execute/quota — si
 * la key es inválida/revocada, el backend responde 401 acá mismo (antes
 * solo se pingeaba /health, que ni siquiera requiere auth, así que nunca
 * confirmaba si la key era válida).
 */
export async function callAI(
  config: AIConfig,
  systemPrompt: string,
  userMessage: string,
): Promise<string> {
  if (config.provider === 'spectreqa_cloud') {
    if (!config.apiKey.trim()) throw new Error('Falta la API key');

    const res = await fetch(`${config.baseUrl}/api/v1/execute/quota`, {
      headers: { Authorization: `Bearer ${config.apiKey}` },
    });

    if (res.status === 401) throw new Error('API key inválida o revocada');
    if (!res.ok) {
      const errorBody = await res.json().catch(() => ({}));
      const detail = errorBody.detail ? ` — ${errorBody.detail}` : '';
      throw new Error(`Backend respondió ${res.status}${detail}`);
    }

    const data = await res.json();
    // El backend devuelve monthlyLimit (mismo valor que dailyLimit, que se
    // mantiene solo por retrocompatibilidad — ver quota.service.ts). La
    // cuota es mensual y apilable desde el rediseño de subscriptions, no
    // diaria; usageDate ya viene en formato 'YYYY-MM', no un día puntual.
    return `Cuota: ${data.phasesUsed}/${data.monthlyLimit} fases usadas este mes`;
  }

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