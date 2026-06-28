// src/components/projects/AgentMdEditor.tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Project } from '../../types/project';

interface AgentMdEditorProps {
  project: Project | null;
  onClose: () => void;
}

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

const AgentMdEditor = ({ project, onClose }: AgentMdEditorProps) => {
  const [content, setContent] = useState('');
  const [original, setOriginal] = useState('');
  const [loading, setLoading] = useState(true);
  const [saveState, setSaveState] = useState<SaveState>('idle');

  useEffect(() => {
    if (!project) return;
    setLoading(true);
    invoke<string>('read_agent_md', { projectId: project.id })
      .then((text) => {
        setContent(text);
        setOriginal(text);
        setLoading(false);
      })
      .catch(() => {
        setContent('');
        setOriginal('');
        setLoading(false);
      });
  }, [project]);

  if (!project) return null;

  const isDirty = content !== original;

  const handleSave = async () => {
    setSaveState('saving');
    try {
      await invoke('write_agent_md', { projectId: project.id, content });
      setOriginal(content);
      setSaveState('saved');
      setTimeout(() => setSaveState('idle'), 2000);
    } catch {
      setSaveState('error');
      setTimeout(() => setSaveState('idle'), 3000);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-xl border border-slate-200 w-full max-w-2xl mx-4 flex flex-col" style={{ height: '80vh' }}>

        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100 shrink-0">
          <div>
            <h2 className="text-base font-semibold text-slate-800">AGENT.md</h2>
            <p className="text-xs text-slate-400 mt-0.5 font-mono">{project.name}</p>
          </div>
          <div className="flex items-center gap-2">
            {isDirty && saveState === 'idle' && (
              <span className="text-xs text-amber-500 font-medium">Sin guardar</span>
            )}
            {saveState === 'saved' && (
              <span className="text-xs text-emerald-600 font-medium">Guardado ✓</span>
            )}
            {saveState === 'error' && (
              <span className="text-xs text-red-500 font-medium">Error al guardar</span>
            )}
            <button
              onClick={handleSave}
              disabled={!isDirty || saveState === 'saving'}
              className="px-3 py-1.5 text-xs font-medium rounded-lg bg-[#534AB7] text-white hover:bg-[#4840a3] disabled:opacity-40 disabled:cursor-not-allowed transition"
            >
              {saveState === 'saving' ? 'Guardando...' : 'Guardar'}
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
        </div>

        {/* Info banner */}
        <div className="px-6 py-2.5 bg-slate-50 border-b border-slate-100 shrink-0">
          <p className="text-xs text-slate-500 leading-relaxed">
            Este archivo contiene las instrucciones para el agente de testing automático.
            Describe qué debe probar, qué elementos ignorar y cualquier contexto relevante.
          </p>
        </div>

        {/* Editor */}
        <div className="flex-1 min-h-0 p-4">
          {loading ? (
            <div className="h-full flex items-center justify-center text-sm text-slate-400">
              Cargando...
            </div>
          ) : (
            <textarea
              value={content}
              onChange={(e) => {
                setContent(e.target.value);
                setSaveState('idle');
              }}
              spellCheck={false}
              className="w-full h-full text-sm font-mono text-slate-700 bg-slate-50 rounded-xl border border-slate-200 px-4 py-3 resize-none focus:outline-none focus:border-[#534AB7] transition leading-relaxed"
              placeholder="# AGENT.md&#10;&#10;Escribe aquí las instrucciones para el agente..."
            />
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-3 border-t border-slate-100 flex items-center justify-between shrink-0">
          <span className="text-xs text-slate-400 font-mono">
            {project.env === 'local'
              ? `~/.local/share/glasstest/projects/${project.id}/AGENT.md`
              : `AppData/glasstest/projects/${project.id}/AGENT.md`}
          </span>
          <span className="text-xs text-slate-400">
            {content.length} caracteres
          </span>
        </div>

      </div>
    </div>
  );
};

export default AgentMdEditor;