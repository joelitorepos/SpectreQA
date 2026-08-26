// src/pages/ProjectsPage.tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useProjects } from '../hooks/useProjects';
import { useExtension } from '../hooks/useExtension';
import ProjectCard from '../components/projects/ProjectCard';
import NewProjectModal from '../components/projects/NewProjectModal';
import EditProjectModal from '../components/projects/EditProjectModal';
import DeleteConfirmModal from '../components/projects/DeleteConfirmModal';
import AgentMdEditor from '../components/projects/AgentMdEditor';
import type { Project } from '../types/project';

const ProjectsPage = () => {
  const { projects, loading, addProject, updateProject, toggleAuditing, removeProject } = useProjects();
  const { connected } = useExtension();

  const [showNew, setShowNew] = useState(false);
  const [toEdit, setToEdit] = useState<Project | null>(null);
  const [toDelete, setToDelete] = useState<Project | null>(null);
  const [agentProject, setAgentProject] = useState<Project | null>(null);

  // Sincronizar con la extensión solo las URLs que el usuario activó explícitamente
  useEffect(() => {
    if (!connected || loading) return;
    const auditingUrls = projects
      .filter((p) => p.auditing)
      .map((p) => p.url);

    invoke('send_to_extension', {
      messageType: 'AUDIT_URL',
      payload: { urls: auditingUrls },
    }).catch(console.error);
  }, [projects, connected, loading]);

  const handleDelete = async () => {
    if (!toDelete) return;
    await removeProject(toDelete.id);
    setToDelete(null);
  };

  return (
    <div className="flex flex-col gap-6 h-full">

      {/* Top bar */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-slate-800">Proyectos</h1>
          <p className="text-sm text-slate-400 mt-0.5">
            {loading ? '...' : `${projects.length} proyecto${projects.length !== 1 ? 's' : ''}`}
            {connected && projects.length > 0 && (
              <span className="ml-2 text-emerald-600">· Extensión conectada</span>
            )}
          </p>
        </div>
        <button
          onClick={() => setShowNew(true)}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-[#534AB7] text-white rounded-xl hover:bg-[#4840a3] transition"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          Nuevo proyecto
        </button>
      </div>

      {/* Extension warning */}
      {!connected && (
        <div className="flex items-center gap-3 bg-amber-50 border border-amber-200 rounded-xl px-4 py-3">
          <span className="text-amber-500 shrink-0">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
              <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </span>
          <p className="text-sm text-amber-700">
            La extensión no está conectada.{' '}
            <a href="https://chromewebstore.google.com" target="_blank" rel="noreferrer"
              className="font-medium underline underline-offset-2">
              Instálala aquí
            </a>{' '}
            para comenzar a auditar.
          </p>
        </div>
      )}

      {/* Content */}
      {loading ? (
        <div className="flex-1 flex items-center justify-center text-slate-400 text-sm">
          Cargando proyectos...
        </div>
      ) : projects.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-3 text-center">
          <div className="w-12 h-12 rounded-2xl bg-slate-100 flex items-center justify-center text-slate-300">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/>
              <rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
            </svg>
          </div>
          <div>
            <p className="text-sm font-medium text-slate-600">Sin proyectos todavía</p>
            <p className="text-sm text-slate-400 mt-0.5">Crea uno para empezar a auditar</p>
          </div>
          <button onClick={() => setShowNew(true)}
            className="mt-2 px-4 py-2 text-sm font-medium bg-[#534AB7] text-white rounded-xl hover:bg-[#4840a3] transition">
            Crear primer proyecto
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {projects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              onEdit={setToEdit}
              onOpenAgent={setAgentProject}
              onToggleAuditing={toggleAuditing}
            />
          ))}
        </div>
      )}

      <NewProjectModal open={showNew} onClose={() => setShowNew(false)} onConfirm={addProject} />
      <EditProjectModal project={toEdit} onClose={() => setToEdit(null)} onConfirm={updateProject} />
      <DeleteConfirmModal project={toDelete} onClose={() => setToDelete(null)} onConfirm={handleDelete} />
      <AgentMdEditor project={agentProject} onClose={() => setAgentProject(null)} />
    </div>
  );
};

export default ProjectsPage;