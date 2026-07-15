// src/pages/SettingsPage.tsx
import { useState, useEffect } from 'react';
import { useSettings } from '../hooks/useSettings';
import type { AIProvider, AIConfig } from '../hooks/useSettings';

const SUGGESTED_MODELS: Record<AIProvider, string[]> = {
  ollama: ['llama3.2', 'llama3.1', 'mistral', 'gemma2', 'qwen2.5'],
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

      {/* Proveedor */}
      <div className="flex flex-col gap-3">
        <label className="text-xs font-medium text-slate-600 uppercase tracking-wide">Proveedor</label>
        <div className="flex">
          <div className="flex items-center gap-4 py-4 px-5 rounded-2xl border border-[#534AB7] bg-[#534AB7]/5 max-w-sm w-full">
            <span className="text-3xl">🦙</span>
            <div className="flex flex-col">
              <span className="text-sm font-semibold text-[#534AB7]">Ollama</span>
              <span className="text-xs text-slate-500 mt-0.5">Modelos locales integrados sin necesidad de API Key.</span>
            </div>
          </div>
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
          disabled={testState === 'testing'}
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