// src/components/projects/EditProjectModal.tsx
import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { Project, ProjectEnv } from '../../types/project';

interface EditProjectModalProps {
  project: Project | null;
  onClose: () => void;
  onConfirm: (updated: Project) => void;
}

const EditProjectModal = ({ project, onClose, onConfirm }: EditProjectModalProps) => {
  const [form, setForm] = useState({
    name: '',
    url: '',
    env: 'local' as ProjectEnv,
    serveCommand: '',
    projectPath: '',
  });

  useEffect(() => {
    if (project) {
      setForm({
        name: project.name,
        url: project.url,
        env: project.env,
        // tomar el primer comando guardado (o vacío si no hay)
        serveCommand: project.commands[0] ?? '',
        projectPath: project.projectPath,
      });
    }
  }, [project]);

  if (!project) return null;

  const handlePickFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Selecciona la carpeta raíz del proyecto',
    });
    if (typeof selected === 'string') {
      setForm((f) => ({ ...f, projectPath: selected }));
    }
  };

  const handleSubmit = () => {
    if (!form.name.trim() || !form.url.trim()) return;
    const commands = form.serveCommand.trim() ? [form.serveCommand.trim()] : [];
    onConfirm({ ...project, ...form, commands });
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-xl border border-slate-200 w-full max-w-md mx-4 p-6 flex flex-col gap-5">

        {/* Header */}
        <div className="flex items-center justify-between">
          <h2 className="text-base font-semibold text-slate-800">Editar proyecto</h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        {/* Nombre */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium text-slate-600">Nombre del proyecto</label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            className="text-sm px-3 py-2 rounded-xl border border-slate-200 bg-slate-50 text-slate-800 focus:outline-none focus:border-[#534AB7] transition"
          />
        </div>

        {/* Entorno */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium text-slate-600">Entorno</label>
          <div className="grid grid-cols-2 gap-2">
            {(['local', 'production'] as ProjectEnv[]).map((env) => (
              <button
                key={env}
                onClick={() => setForm({ ...form, env })}
                className={`flex flex-col items-center gap-1.5 py-3 rounded-xl border text-sm font-medium transition ${
                  form.env === env
                    ? 'border-[#534AB7] bg-[#534AB7]/5 text-[#534AB7]'
                    : 'border-slate-200 bg-slate-50 text-slate-500 hover:border-slate-300'
                }`}
              >
                <span className="text-lg">{env === 'local' ? '🏠' : '🌐'}</span>
                {env === 'local' ? 'Local' : 'Producción'}
              </button>
            ))}
          </div>
        </div>

        {/* URL */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium text-slate-600">
            {form.env === 'local' ? 'Host y puerto' : 'Dominio'}
          </label>
          <div className="flex items-center border border-slate-200 bg-slate-50 rounded-xl overflow-hidden focus-within:border-[#534AB7] transition">
            <span className="px-3 text-xs text-slate-400 select-none">
              {form.env === 'local' ? 'http://' : 'https://'}
            </span>
            <input
              type="text"
              value={form.url}
              onChange={(e) => setForm({ ...form, url: e.target.value })}
              className="flex-1 text-sm py-2 pr-3 bg-transparent text-slate-800 font-mono focus:outline-none"
            />
          </div>
        </div>

        {/* Solo para local: carpeta raíz + comando */}
        {form.env === 'local' && (
          <>
            {/* Carpeta raíz */}
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-medium text-slate-600">
                Carpeta raíz del proyecto
              </label>
              <div className="flex gap-2">
                <div className="flex-1 flex items-center border border-slate-200 bg-slate-50 rounded-xl overflow-hidden focus-within:border-[#534AB7] transition min-w-0">
                  <span className="pl-3 shrink-0">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#94a3b8" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
                    </svg>
                  </span>
                  <input
                    type="text"
                    readOnly
                    placeholder="/home/usuario/mi-proyecto"
                    value={form.projectPath}
                    className="flex-1 text-xs py-2 px-2 bg-transparent text-slate-700 placeholder:text-slate-400 focus:outline-none font-mono truncate cursor-default"
                  />
                </div>
                <button
                  onClick={handlePickFolder}
                  className="shrink-0 px-3 py-2 text-xs font-medium rounded-xl border border-slate-200 text-slate-600 hover:border-[#534AB7] hover:text-[#534AB7] transition"
                >
                  Explorar
                </button>
              </div>
            </div>

            {/* Comando para servir */}
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-medium text-slate-600">
                Comando para levantar el proyecto{' '}
                <span className="font-normal text-slate-400">(opcional)</span>
              </label>
              <div className="flex items-center border border-slate-200 bg-slate-50 rounded-xl overflow-hidden focus-within:border-[#534AB7] transition">
                <span className="pl-3 text-xs text-slate-400 select-none font-mono">$</span>
                <input
                  type="text"
                  placeholder="npm run dev"
                  value={form.serveCommand}
                  onChange={(e) => setForm({ ...form, serveCommand: e.target.value })}
                  className="flex-1 text-xs py-2 px-2 bg-transparent text-slate-700 placeholder:text-slate-400 focus:outline-none font-mono"
                />
              </div>
              {form.projectPath && (
                <p className="text-[11px] text-slate-400">
                  Se ejecutará desde{' '}
                  <span className="font-mono text-slate-500">{form.projectPath}</span>
                </p>
              )}
            </div>
          </>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-1">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-xl border border-slate-200 text-slate-500 hover:bg-slate-50 transition"
          >
            Cancelar
          </button>
          <button
            onClick={handleSubmit}
            disabled={!form.name.trim() || !form.url.trim()}
            className="px-4 py-2 text-sm font-medium rounded-xl bg-[#534AB7] text-white hover:bg-[#4840a3] disabled:opacity-40 disabled:cursor-not-allowed transition"
          >
            Guardar cambios
          </button>
        </div>

      </div>
    </div>
  );
};

export default EditProjectModal;