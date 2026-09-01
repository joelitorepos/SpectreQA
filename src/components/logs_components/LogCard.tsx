// src/components/logs/LogCard.tsx
import { useEffect, useState } from 'react';
import type { TestLogEntry, TestLogSummary } from '../../types/testLog';

interface LogCardProps {
  summary: TestLogSummary;
  /** Identificador que ve el QA: qué se probó (normalmente la URL del proyecto). */
  pageLabel: string;
  readLog: (logId: string) => Promise<TestLogEntry | null>;
  onDelete: (logId: string) => void;
}

const OUTCOME_STYLES: Record<TestLogSummary['outcome'], { label: string; badge: string }> = {
  success: { label: 'Éxito', badge: 'bg-emerald-50 text-emerald-700 border-emerald-200' },
  error: { label: 'Error', badge: 'bg-red-50 text-red-700 border-red-200' },
  terminated: { label: 'Detenido', badge: 'bg-amber-50 text-amber-700 border-amber-200' },
};

const formatDateTime = (unixSeconds: number) => {
  const date = new Date(unixSeconds * 1000);
  return new Intl.DateTimeFormat('es', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
};

const LogCard = ({ summary, pageLabel, readLog, onDelete }: LogCardProps) => {
  const [entry, setEntry] = useState<TestLogEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    readLog(summary.log_id).then((result) => {
      if (!cancelled) {
        setEntry(result);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [summary.log_id, readLog]);

  const outcome = OUTCOME_STYLES[summary.outcome];

  return (
    <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
      {/* Header: identificador (página + fecha/hora) + resultado + eliminar */}
      <div className="flex items-center justify-between gap-3 px-4 py-3 border-b border-slate-100">
        <div className="min-w-0">
          <p className="text-sm font-medium text-slate-800 truncate">{pageLabel}</p>
          <p className="text-xs text-slate-400 mt-0.5">{formatDateTime(summary.started_at)}</p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className={`text-xs font-medium px-2 py-1 rounded-full border ${outcome.badge}`}>
            {outcome.label}
          </span>

          {confirmingDelete ? (
            <div className="flex items-center gap-1">
              <button
                onClick={() => onDelete(summary.log_id)}
                className="text-xs font-medium text-white bg-red-500 hover:bg-red-600 px-2 py-1 rounded-lg transition"
              >
                Confirmar
              </button>
              <button
                onClick={() => setConfirmingDelete(false)}
                className="text-xs font-medium text-slate-500 hover:text-slate-700 px-2 py-1 rounded-lg transition"
              >
                Cancelar
              </button>
            </div>
          ) : (
            <button
              onClick={() => setConfirmingDelete(true)}
              className="text-slate-400 hover:text-red-500 transition p-1"
              title="Eliminar log"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                <path d="M10 11v6" /><path d="M14 11v6" />
                <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Cuerpo: fases con pensamiento (azul) y comandos (verde) */}
      <div className="p-4">
        {loading ? (
          <p className="text-sm text-slate-400">Cargando fases...</p>
        ) : !entry ? (
          <p className="text-sm text-red-500">No se pudo cargar el detalle de este log.</p>
        ) : entry.phases.length === 0 ? (
          <p className="text-sm text-slate-400">Esta prueba terminó sin registrar fases.</p>
        ) : (
          <div className="flex flex-col gap-4">
            {entry.phases.map((phase) => (
              <div key={phase.phase}>
                <div className="flex items-center gap-2 mb-2">
                  <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">
                    Fase {phase.phase}
                  </span>
                  <div className="h-px flex-1 bg-slate-100" />
                </div>

                {phase.thought && (
                  <div className="bg-blue-50 border border-blue-100 text-blue-800 text-sm rounded-lg px-3 py-2 mb-2">
                    {phase.thought}
                  </div>
                )}

                {phase.commands.length > 0 && (
                  <div className="flex flex-col gap-1.5">
                    {phase.commands.map((command, idx) => (
                      <div
                        key={idx}
                        className="bg-emerald-50 border border-emerald-100 text-emerald-800 text-sm font-mono rounded-lg px-3 py-1.5"
                      >
                        {command}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default LogCard;