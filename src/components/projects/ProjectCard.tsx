// src/components/projects/ProjectCard.tsx
import { useState } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import type { Project } from '../../types/project';

interface ProjectCardProps {
  project: Project;
  onDelete: (project: Project) => void;
  onEdit: (project: Project) => void;
  onOpenAgent: (project: Project) => void;
  onToggleAuditing: (id: string) => void;
}

type RunState = 'idle' | 'running' | 'error';

const IconEdit = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
  </svg>
);

const IconFile = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
    <polyline points="14 2 14 8 20 8"/>
    <line x1="16" y1="13" x2="8" y2="13"/>
    <line x1="16" y1="17" x2="8" y2="17"/>
  </svg>
);

const IconTrash = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/>
    <path d="M10 11v6m4-6v6"/><path d="M9 6V4h6v2"/>
  </svg>
);

// Toggle switch component
const Toggle = ({ checked, onChange }: { checked: boolean; onChange: () => void }) => (
  <button
    onClick={(e) => { e.stopPropagation(); onChange(); }}
    title={checked ? 'Desactivar auditoría' : 'Activar auditoría'}
    className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors duration-200 focus:outline-none ${
      checked ? 'bg-[#534AB7]' : 'bg-slate-200'
    }`}
  >
    <span className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform duration-200 ${
      checked ? 'translate-x-4.5' : 'translate-x-0.5'
    }`} />
  </button>
);

const ProjectCard = ({ project, onDelete, onEdit, onOpenAgent, onToggleAuditing }: ProjectCardProps) => {
  const [runState, setRunState] = useState<RunState>('idle');
  const [errorMsg, setErrorMsg] = useState('');

  const handleCardClick = async (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
  
    const url = project.env === 'local'
      ? `http://${project.url}`
      : `https://${project.url}`;
  
    // Producción o sin comando: abrir directo
    if (project.env === 'production' || project.commands.length === 0) {
      await openUrl(url);
      return;
    }
  
    setRunState('running');
    setErrorMsg('');
  
    try {
      await invoke('run_project_commands', {
        command: project.commands[0],   // ← ahora es un string, no array
        cwd: project.projectPath,
      });
      setRunState('idle');
      await openUrl(url);
    } catch (err) {
      setRunState('error');
      setErrorMsg(typeof err === 'string' ? err : 'Error ejecutando el comando');
      setTimeout(() => { setRunState('idle'); setErrorMsg(''); }, 5000);
    }
  };


  return (
    <div
      onClick={handleCardClick}
      className={`bg-white border rounded-2xl p-5 flex flex-col gap-4 cursor-pointer transition-all duration-150 group ${
        runState === 'error'
          ? 'border-red-200 bg-red-50/30'
          : project.auditing
          ? 'border-[#534AB7]/30 shadow-sm shadow-[#534AB7]/10'
          : 'border-slate-200 hover:border-slate-300 hover:shadow-sm'
      }`}
    >
      {/* Header */}
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="font-medium text-slate-800 text-sm truncate">{project.name}</p>
          <p className="text-xs text-slate-400 font-mono mt-0.5 truncate">{project.url}</p>
        </div>
        <span className={`shrink-0 text-[11px] font-medium px-2.5 py-1 rounded-full ${
          project.env === 'local' ? 'bg-amber-50 text-amber-700' : 'bg-blue-50 text-blue-700'
        }`}>
          {project.env === 'local' ? 'Local' : 'Producción'}
        </span>
      </div>

      {/* Commands */}
      {project.commands.length > 0 && (
        <div className="bg-slate-50 rounded-xl px-3 py-2.5 space-y-1">
          {project.commands.map((cmd, i) => (
            <p key={i} className="text-[11px] font-mono text-slate-500 truncate">
              <span className="text-slate-300 mr-1.5">$</span>{cmd}
            </p>
          ))}
        </div>
      )}

      {/* Error */}
      {runState === 'error' && errorMsg && (
        <div className="bg-red-50 border border-red-100 rounded-xl px-3 py-2">
          <p className="text-[11px] text-red-600 font-mono leading-relaxed line-clamp-3">{errorMsg}</p>
        </div>
      )}

      {/* Footer */}
      <div className="flex items-center justify-between pt-1 border-t border-slate-100">

        {/* Auditing toggle */}
        <div className="flex items-center gap-2">
          {runState === 'running' ? (
            <>
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse shrink-0" />
              <span className="text-xs text-amber-600">Iniciando...</span>
            </>
          ) : runState === 'error' ? (
            <>
              <span className="w-1.5 h-1.5 rounded-full bg-red-400 shrink-0" />
              <span className="text-xs text-red-500">Error al iniciar</span>
            </>
          ) : (
            <>
              <Toggle
                checked={project.auditing}
                onChange={() => onToggleAuditing(project.id)}
              />
              <span className={`text-xs font-medium ${project.auditing ? 'text-[#534AB7]' : 'text-slate-400'}`}>
                {project.auditing ? 'Auditando' : 'En espera'}
              </span>
            </>
          )}
        </div>

        {/* Action buttons */}
        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-all duration-150">
          <button
            onClick={(e) => { e.stopPropagation(); onOpenAgent(project); }}
            className="p-1.5 rounded-lg text-slate-400 hover:text-[#534AB7] hover:bg-[#534AB7]/8 transition"
            title="Editar AGENT.md"
          >
            <IconFile />
          </button>
          <button
            onClick={(e) => { e.stopPropagation(); onEdit(project); }}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 hover:bg-slate-100 transition"
            title="Editar proyecto"
          >
            <IconEdit />
          </button>
          <button
            onClick={(e) => { e.stopPropagation(); onDelete(project); }}
            className="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-50 transition"
            title="Eliminar proyecto"
          >
            <IconTrash />
          </button>
        </div>
      </div>
    </div>
  );
};

export default ProjectCard;