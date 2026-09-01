// src/pages/LogsPage.tsx
import { useEffect, useState } from 'react';
import { useProjects } from '../hooks/useProjects';
import { useTestLogs } from '../hooks/useTestLogs';
import LogCard from '../components/logs_components/LogCard';

const LogsPage = () => {
  const { projects, loading: loadingProjects } = useProjects();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Preselecciona el proyecto activo si hay uno, si no el primero de la lista.
  useEffect(() => {
    if (selectedId || projects.length === 0) return;
    const active = projects.find((p) => p.auditing);
    setSelectedId(active?.id ?? projects[0].id);
  }, [projects, selectedId]);

  const selectedProject = projects.find((p) => p.id === selectedId) ?? null;
  const { logs, loading: loadingLogs, readLog, deleteLog } = useTestLogs(selectedProject?.id ?? null);

  // Identificador que pide el QA: a qué página se le corrió la prueba.
  const pageLabel = selectedProject ? (selectedProject.url || selectedProject.name) : '';

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-slate-800">Logs</h1>
          <p className="text-sm text-slate-400 mt-0.5">Historial de pruebas por proyecto</p>
        </div>

        {!loadingProjects && projects.length > 0 && (
          <select
            value={selectedId ?? ''}
            onChange={(e) => setSelectedId(e.target.value)}
            className="text-sm border border-slate-200 rounded-lg px-3 py-2 text-slate-700 bg-white focus:outline-none focus:ring-2 focus:ring-[#534AB7]/30"
          >
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {loadingProjects ? (
        <div className="flex-1 flex items-center justify-center text-slate-400 text-sm">
          Cargando proyectos...
        </div>
      ) : projects.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-3 text-center">
          <div className="w-12 h-12 rounded-2xl bg-slate-100 flex items-center justify-center text-slate-300">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          </div>
          <div>
            <p className="text-sm font-medium text-slate-600">No hay proyectos todavía</p>
            <p className="text-sm text-slate-400 mt-0.5">Crea un proyecto para empezar a ver logs</p>
          </div>
        </div>
      ) : loadingLogs ? (
        <div className="flex-1 flex items-center justify-center text-slate-400 text-sm">
          Cargando logs...
        </div>
      ) : logs.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-3 text-center">
          <div className="w-12 h-12 rounded-2xl bg-slate-100 flex items-center justify-center text-slate-300">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          </div>
          <div>
            <p className="text-sm font-medium text-slate-600">Sin logs todavía</p>
            <p className="text-sm text-slate-400 mt-0.5">Corre una prueba para que aparezca aquí</p>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          {logs.map((log) => (
            <LogCard
              key={log.log_id}
              summary={log}
              pageLabel={pageLabel}
              readLog={readLog}
              onDelete={deleteLog}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default LogsPage;