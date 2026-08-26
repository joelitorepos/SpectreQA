// src/types/testLog.ts

/** Coincide con TestOutcome en session.rs (serde rename_all = "lowercase"). */
export type TestOutcome = 'success' | 'error' | 'terminated';

/** Coincide con PhaseRecord en session.rs. */
export interface PhaseRecord {
  phase: number;
  thought: string;
  status: string;
  commands: string[];
}

/**
 * Lo que devuelve list_test_logs: liviano a propósito (Rust lo arma leyendo
 * solo los nombres de archivo, sin abrir cada JSON). Suficiente para pintar
 * la lista antes de pedir el detalle de cada uno.
 */
export interface TestLogSummary {
  log_id: string;
  outcome: TestOutcome;
  started_at: number; // unix, segundos
}

/** Lo que devuelve read_test_log: el detalle completo con fases. */
export interface TestLogEntry {
  project_id: string;
  outcome: TestOutcome;
  started_at: number;
  finished_at: number;
  phases: PhaseRecord[];
}