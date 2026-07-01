// src/hooks/useSettings.ts
import { useEffect, useState } from 'react';
import { load, Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';

export type AIProvider = 'openai' | 'anthropic' | 'ollama';

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
      const store = await getStore();
      const stored = await store.get<Settings>(STORE_KEY);
      const loaded = stored ?? DEFAULT_SETTINGS;
      setSettings(loaded);
      setLoading(false);

      // Sincronizar con Rust al arrancar la app por si ya había config guardada
      await syncAiConfig(loaded.ai);
    };
    init();
  }, []);

  const save = async (updated: Settings) => {
    setSaving(true);
    const store = await getStore();
    await store.set(STORE_KEY, updated);
    await store.save();
    setSettings(updated);

    // Notificar a Rust para que use la nueva config en la próxima prueba
    await syncAiConfig(updated.ai);

    setSaving(false);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return { settings, loading, saving, saved, save };
};

/**
 * Envía la AIConfig al backend de Rust.
 * Se llama al arrancar la app (para restaurar config guardada)
 * y cada vez que el usuario guarda cambios en SettingsPage.
 */
async function syncAiConfig(config: AIConfig): Promise<void> {
  try {
    await invoke('set_ai_config', {
      provider: config.provider,
      model: config.model,
      apiKey: config.apiKey,
      baseUrl: config.baseUrl,
    });
  } catch (e) {
    console.error('[SpectreQA] Error sincronizando AIConfig con Rust:', e);
  }
}

/**
 * Función de utilidad para llamar a la IA desde cualquier parte de la app.
 * Usada por SettingsPage para probar la conexión.
 */
export async function callAI(
  config: AIConfig,
  systemPrompt: string,
  userMessage: string,
): Promise<string> {
  if (config.provider === 'anthropic') {
    const res = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': config.apiKey,
        'anthropic-version': '2023-06-01',
      },
      body: JSON.stringify({
        model: config.model,
        max_tokens: 1024,
        system: systemPrompt,
        messages: [{ role: 'user', content: userMessage }],
      }),
    });
    if (!res.ok) throw new Error(`Anthropic error: ${res.status}`);
    const data = await res.json();
    return data.content[0].text;
  }

  // OpenAI o Ollama (misma estructura de chat completions)
  const baseUrl = config.provider === 'ollama'
    ? config.baseUrl
    : 'https://api.openai.com';

  const res = await fetch(`${baseUrl}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(config.apiKey ? { Authorization: `Bearer ${config.apiKey}` } : {}),
    },
    body: JSON.stringify({
      model: config.model,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userMessage },
      ],
    }),
  });
  if (!res.ok) throw new Error(`${config.provider} error: ${res.status}`);
  const data = await res.json();
  return data.choices[0].message.content;
}