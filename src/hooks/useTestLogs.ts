// src/hooks/useTestLogs.ts
import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { TestLogSummary, TestLogEntry } from '../types/testLog';

/**
 * Logs de prueba de un proyecto específico.
 * list_test_logs es barato de llamar (Rust solo lee nombres de archivo), así
 * que refrescar la lista completa está bien. El detalle con fases se pide
 * aparte con readLog, bajo demanda por cada card, para no cargar todo de una
 * si hay muchos logs.
 */
export const useTestLogs = (projectId: string | null) => {
  const [logs, setLogs] = useState<TestLogSummary[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setLogs([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<TestLogSummary[]>('list_test_logs', { projectId });
      setLogs(result);
    } catch (e) {
      console.error('[SpectreQA] Error listando logs:', e);
      setLogs([]);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const readLog = useCallback(
    async (logId: string): Promise<TestLogEntry | null> => {
      if (!projectId) return null;
      try {
        return await invoke<TestLogEntry>('read_test_log', { projectId, logId });
      } catch (e) {
        console.error('[SpectreQA] Error leyendo log:', logId, e);
        return null;
      }
    },
    [projectId]
  );

  const deleteLog = useCallback(
    async (logId: string) => {
      if (!projectId) return;
      try {
        await invoke('delete_test_log', { projectId, logId });
        // Actualización optimista: ya sabemos qué se borró, no hace falta
        // volver a pedirle la lista completa a Rust.
        setLogs((prev) => prev.filter((l) => l.log_id !== logId));
      } catch (e) {
        console.error('[SpectreQA] Error eliminando log:', logId, e);
      }
    },
    [projectId]
  );

  return { logs, loading, refresh, readLog, deleteLog };
};