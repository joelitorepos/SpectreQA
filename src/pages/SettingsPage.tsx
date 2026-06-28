// src/pages/SettingsPage.tsx
import { useState, useEffect } from 'react';
import { useSettings } from '../hooks/useSettings';
import type { AIProvider, AIConfig } from '../hooks/useSettings';

const PROVIDERS: { id: AIProvider; label: string; icon: string; description: string }[] = [
  {
    id: 'ollama',
    label: 'Ollama',
    icon: '🦙',
    description: 'Modelos locales, sin API key',
  },
  {
    id: 'openai',
    label: 'OpenAI',
    icon: '⚡',
    description: 'GPT-4o, GPT-4 Turbo, etc.',
  },
  {
    id: 'anthropic',
    label: 'Anthropic',
    icon: '◆',
    description: 'Claude 3.5, Claude 3 Opus, etc.',
  },
];

const SUGGESTED_MODELS: Record<AIProvider, string[]> = {
  ollama: ['llama3.2', 'llama3.1', 'mistral', 'gemma2', 'qwen2.5'],
  openai: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
  anthropic: ['claude-opus-4-5', 'claude-sonnet-4-5', 'claude-haiku-4-5'],
};

const SettingsPage = () => {
  const { settings, loading, saving, saved, save } = useSettings();
  const [form, setForm] = useState<AIConfig>(settings.ai);
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [ollamaStatus, setOllamaStatus] = useState<'checking' | 'online' | 'offline'>('checking');
  const [testState, setTestState] = useState<'idle' | 'testing' | 'ok' | 'error'>('idle');
  const [testError, setTestError] = useState('');

  // Sincronizar form cuando carga el store
  useEffect(() => {
    if (!loading) setForm(settings.ai);
  }, [loading]);

  // Detectar Ollama y listar modelos disponibles
  useEffect(() => {
    if (form.provider !== 'ollama') return;
    setOllamaStatus('checking');
    fetch(`${form.baseUrl}/api/tags`)
      .then((r) => r.json())
      .then((data) => {
        const names = (data.models ?? []).map((m: { name: string }) =>
          m.name.replace(/:latest$/, '')
        );
        setOllamaModels(names);
        setOllamaStatus('online');
      })
      .catch(() => {
        setOllamaModels([]);
        setOllamaStatus('offline');
      });
  }, [form.provider, form.baseUrl]);

  const handleProviderChange = (provider: AIProvider) => {
    setForm({
      provider,
      apiKey: '',
      model: SUGGESTED_MODELS[provider][0],
      baseUrl: provider === 'ollama' ? 'http://localhost:11434' : '',
    });
    setTestState('idle');
  };

  const handleSave = () => save({ ...settings, ai: form });

  const handleTest = async () => {
    setTestState('testing');
    setTestError('');
    try {
      const { callAI } = await import('../hooks/useSettings');
      await callAI(form, 'Responde solo con "ok".', 'Test de conexión');
      setTestState('ok');
    } catch (e) {
      setTestState('error');
      setTestError(e instanceof Error ? e.message : String(e));
    }
  };

  const isDirty = JSON.stringify(form) !== JSON.stringify(settings.ai);

  const modelList = form.provider === 'ollama' && ollamaModels.length > 0
    ? ollamaModels
    : SUGGESTED_MODELS[form.provider];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-slate-400 text-sm">
        Cargando configuración...
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8 max-w-xl">
      <div>
        <h1 className="text-lg font-semibold text-slate-800">Configuración</h1>
        <p className="text-sm text-slate-400 mt-0.5">Modelo de IA para el agente de testing</p>
      </div>

      {/* Provider selector */}
      <div className="flex flex-col gap-3">
        <label className="text-xs font-medium text-slate-600 uppercase tracking-wide">Proveedor</label>
        <div className="grid grid-cols-3 gap-2">
          {PROVIDERS.map((p) => (
            <button
              key={p.id}
              onClick={() => handleProviderChange(p.id)}
              className={`flex flex-col items-center gap-2 py-4 px-3 rounded-2xl border text-center transition ${
                form.provider === p.id
                  ? 'border-[#534AB7] bg-[#534AB7]/5'
                  : 'border-slate-200 bg-white hover:border-slate-300'
              }`}
            >
              <span className="text-2xl">{p.icon}</span>
              <span className={`text-sm font-medium ${form.provider === p.id ? 'text-[#534AB7]' : 'text-slate-700'}`}>
                {p.label}
              </span>
              <span className="text-[11px] text-slate-400 leading-tight">{p.description}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Ollama status */}
      {form.provider === 'ollama' && (
        <div className={`flex items-center gap-3 px-4 py-3 rounded-xl border text-sm ${
          ollamaStatus === 'online'
            ? 'bg-emerald-50 border-emerald-200 text-emerald-700'
            : ollamaStatus === 'offline'
            ? 'bg-red-50 border-red-200 text-red-600'
            : 'bg-slate-50 border-slate-200 text-slate-500'
        }`}>
          <span className={`w-2 h-2 rounded-full shrink-0 ${
            ollamaStatus === 'online' ? 'bg-emerald-500'
            : ollamaStatus === 'offline' ? 'bg-red-400'
            : 'bg-slate-300 animate-pulse'
          }`} />
          {ollamaStatus === 'online'
            ? `Ollama detectado · ${ollamaModels.length} modelo${ollamaModels.length !== 1 ? 's' : ''} disponible${ollamaModels.length !== 1 ? 's' : ''}`
            : ollamaStatus === 'offline'
            ? 'Ollama no detectado — ¿está corriendo? Ejecuta: ollama serve'
            : 'Detectando Ollama...'}
        </div>
      )}

      {/* Ollama base URL */}
      {form.provider === 'ollama' && (
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium text-slate-600">URL de Ollama</label>
          <input
            type="text"
            value={form.baseUrl}
            onChange={(e) => setForm({ ...form, baseUrl: e.target.value })}
            className="text-sm font-mono px-3 py-2 rounded-xl border border-slate-200 bg-slate-50 text-slate-800 focus:outline-none focus:border-[#534AB7] transition"
          />
        </div>
      )}

      {/* API Key — solo para openai y anthropic */}
      {form.provider !== 'ollama' && (
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium text-slate-600">
            API Key{' '}
            <a
              href={form.provider === 'openai'
                ? 'https://platform.openai.com/api-keys'
                : 'https://console.anthropic.com/settings/keys'}
              target="_blank"
              rel="noreferrer"
              className="font-normal text-[#534AB7] underline underline-offset-2"
            >
              Obtener →
            </a>
          </label>
          <input
            type="password"
            placeholder={form.provider === 'openai' ? 'sk-...' : 'sk-ant-...'}
            value={form.apiKey}
            onChange={(e) => setForm({ ...form, apiKey: e.target.value })}
            className="text-sm font-mono px-3 py-2 rounded-xl border border-slate-200 bg-slate-50 text-slate-800 placeholder:text-slate-400 focus:outline-none focus:border-[#534AB7] transition"
          />
          <p className="text-[11px] text-slate-400">
            La API key se guarda localmente en tu máquina, nunca se envía a ningún servidor externo salvo al proveedor.
          </p>
        </div>
      )}

      {/* Modelo */}
      <div className="flex flex-col gap-1.5">
        <label className="text-xs font-medium text-slate-600">Modelo</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={form.model}
            onChange={(e) => setForm({ ...form, model: e.target.value })}
            className="flex-1 text-sm font-mono px-3 py-2 rounded-xl border border-slate-200 bg-slate-50 text-slate-800 focus:outline-none focus:border-[#534AB7] transition"
            placeholder="nombre del modelo"
          />
        </div>
        {/* Sugerencias */}
        <div className="flex flex-wrap gap-1.5 mt-1">
          {modelList.map((m) => (
            <button
              key={m}
              onClick={() => setForm({ ...form, model: m })}
              className={`text-[11px] px-2.5 py-1 rounded-lg border transition ${
                form.model === m
                  ? 'border-[#534AB7] bg-[#534AB7]/8 text-[#534AB7] font-medium'
                  : 'border-slate-200 text-slate-500 hover:border-slate-300'
              }`}
            >
              {m}
            </button>
          ))}
        </div>
      </div>

      {/* Test + Save */}
      <div className="flex items-center gap-3 pt-2">
        <button
          onClick={handleTest}
          disabled={testState === 'testing' || (form.provider !== 'ollama' && !form.apiKey)}
          className="px-4 py-2 text-sm rounded-xl border border-slate-200 text-slate-600 hover:border-slate-300 disabled:opacity-40 disabled:cursor-not-allowed transition"
        >
          {testState === 'testing' ? 'Probando...' : 'Probar conexión'}
        </button>

        {testState === 'ok' && (
          <span className="text-sm text-emerald-600 font-medium">Conexión exitosa ✓</span>
        )}
        {testState === 'error' && (
          <span className="text-sm text-red-500 truncate max-w-xs" title={testError}>
            Error: {testError}
          </span>
        )}

        <div className="ml-auto flex items-center gap-3">
          {saved && <span className="text-xs text-emerald-600 font-medium">Guardado ✓</span>}
          <button
            onClick={handleSave}
            disabled={!isDirty || saving}
            className="px-4 py-2 text-sm font-medium rounded-xl bg-[#534AB7] text-white hover:bg-[#4840a3] disabled:opacity-40 disabled:cursor-not-allowed transition"
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </button>
        </div>
      </div>

    </div>
  );
};

export default SettingsPage;