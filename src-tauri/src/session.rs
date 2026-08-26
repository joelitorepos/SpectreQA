// src-tauri/src/session.rs

use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/**
 * Registro de una fase completada.
 * Se guarda en respuestas.json y se pasa al prompt_builder para construir el historial.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseRecord {
    pub phase: u32,
    pub thought: String,
    pub status: String,
    pub commands: Vec<String>,
}

/**
 * Estado posible de una sesión de prueba (uso interno, mientras la sesión vive).
 */
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Running,
    Finished,
    Error,
}

/**
 * Por qué terminó una sesión. Esto es lo que se guarda en el log — a
 * diferencia de SessionStatus (que es un detalle interno mientras la sesión
 * vive), TestOutcome es el resultado final que le importa al usuario.
 */
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    Success,
    Error,
    Terminated,
}

impl TestOutcome {
    /// Usado para nombrar el archivo del log: log_{timestamp}_{outcome}.json
    pub fn as_filename_part(&self) -> &'static str {
        match self {
            TestOutcome::Success => "success",
            TestOutcome::Error => "error",
            TestOutcome::Terminated => "terminated",
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/**
 * Lo que se guarda en disco al terminar una prueba (éxito, error, o detenida
 * por el usuario). Contiene exactamente lo que pediste: número de fase,
 * pensamiento de la IA y comandos usados por fase (Vec<PhaseRecord>, lo mismo
 * que ya se usaba para el historial en memoria), más el resultado final.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLogEntry {
    pub project_id: String,
    pub outcome: TestOutcome,
    pub started_at: u64,
    pub finished_at: u64,
    pub phases: Vec<PhaseRecord>,
}

/**
 * Versión liviana de TestLogEntry para listar logs sin tener que abrir y
 * parsear cada archivo JSON: toda esta info sale directo del nombre del
 * archivo (log_{started_at}_{outcome}.json).
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLogSummary {
    pub log_id: String,
    pub outcome: TestOutcome,
    pub started_at: u64,
}

/**
 * Sesión de prueba activa para un proyecto.
 * Vive en AppState.test_session (Mutex<Option<TestSession>>).
 * Se crea al recibir START_TEST y se limpia al terminar.
 */
#[derive(Debug, Clone)]
pub struct TestSession {
    pub project_id: String,
    pub agent_md: String,
    pub current_phase: u32,
    pub history: Vec<PhaseRecord>,
    pub last_dom: Option<Value>,
    pub status: SessionStatus,
    /** Solo para logging — la prueba termina en el primer DOM sin cambios, sin reintentos. */
    pub no_change_retries: u32,
    /** Última URL conocida para detectar navegación (login exitoso) */
    pub last_url: Option<String>,
    /** Marca de tiempo del último EXECUTE_PHASE enviado al cliente */
    pub last_phase_sent_at: Option<Instant>,
    // NUEVOS CAMPOS PARA CONTROL DE NAVEGACION
    pub is_navigating: bool,
    pub last_navigation_time: Option<u64>,
    pub navigation_timeout_secs: u64,
    /**
     * Flag que indica si la sesión está en espera asíncrona controlada (ASYNC_WAIT).
     * Cuando es true, el watchdog no debe actuar, ya que la pausa es intencional.
     */
    pub is_async_waiting: bool,
    /** Marca de tiempo (unix, segundos) de cuándo se creó la sesión. Para el log. */
    pub started_at: u64,
}

impl TestSession {
    /**
     * Crea una nueva sesión desde cero para el proyecto dado.
     * agent_md es el contenido del AGENT.md leído de disco.
     * history es el historial previo cargado de respuestas.json (puede ser vacío).
     */
    pub fn new(project_id: String, agent_md: String, history: Vec<PhaseRecord>) -> Self {
        Self {
            project_id,
            agent_md,
            current_phase: 0,
            history,
            last_dom: None,
            status: SessionStatus::Running,
            no_change_retries: 0,
            last_url: None,
            last_phase_sent_at: None,
            // NUEVOS CAMPOS INICIALIZADOS
            is_navigating: false,
            last_navigation_time: None,
            navigation_timeout_secs: 30, // 30 segundos máximo para navegación
            is_async_waiting: false,     // Inicialmente no está en espera asíncrona
            started_at: now_unix(),
        }
    }

    /**
     * Arma el registro de log a partir de esta sesión y el resultado final.
     * No toca disco: eso lo hace storage::save_test_log con lo que devuelve esto.
     */
    pub fn to_log_entry(&self, outcome: TestOutcome) -> TestLogEntry {
        TestLogEntry {
            project_id: self.project_id.clone(),
            outcome,
            started_at: self.started_at,
            finished_at: now_unix(),
            phases: self.history.clone(),
        }
    }

    /**
     * Agrega un PhaseRecord al historial en memoria.
     * El guardado a disco lo hace storage::append_respuesta por separado.
     */
    pub fn add_phase(&mut self, record: PhaseRecord) {
        self.history.push(record);
    }

    /**
     * Avanza al siguiente número de fase y resetea el contador de reintentos.
     */
    pub fn advance_phase(&mut self) {
        self.current_phase += 1;
        self.no_change_retries = 0;
    }

    /**
     * Registra que el DOM no cambió respecto al snapshot anterior. Ya NO
     * hay reintentos — un solo "sin cambios" termina la prueba con error
     * (ver ws_server.rs::handle_dom_snapshot). Este contador queda solo
     * para logging, no gatea ninguna decisión.
     */
    pub fn register_no_change(&mut self) -> bool {
        self.no_change_retries += 1;
        self.no_change_retries > 2
    }

    /**
     * Actualiza el último snapshot DOM conocido.
     */
    pub fn set_last_dom(&mut self, dom: Value) {
        self.last_dom = Some(dom);
    }

    /**
     * Marca que se ha enviado un EXECUTE_PHASE al cliente.
     * Actualiza la marca de tiempo con el instante actual.
     */
    pub fn mark_phase_sent(&mut self) {
        self.last_phase_sent_at = Some(Instant::now());
    }

    /**
     * Limpia la marca de tiempo (útil cuando se limpia la sesión).
     */
    pub fn clear_phase_sent(&mut self) {
        self.last_phase_sent_at = None;
    }

    /**
     * Verifica si ha pasado más de `timeout_secs` segundos desde el último envío.
     * Devuelve true si se ha superado el timeout o si no hay marca de tiempo.
     */
    pub fn is_phase_timeout(&self, timeout_secs: u64) -> bool {
        match self.last_phase_sent_at {
            Some(timestamp) => timestamp.elapsed() > std::time::Duration::from_secs(timeout_secs),
            None => false, // No hay marca de tiempo, no consideramos timeout
        }
    }

    /**
     * Marca el inicio de una navegación.
     */
    pub fn start_navigation(&mut self) {
        self.is_navigating = true;
        self.last_navigation_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        println!("[SpectreQA] Navegación iniciada. Modo de espera activado.");
    }

    /**
     * Marca el fin de una navegación (cuando se recibe DOM_SNAPSHOT).
     */
    pub fn end_navigation(&mut self) {
        self.is_navigating = false;
        self.last_navigation_time = None;
        println!("[SpectreQA] Navegación completada. Prueba reanudada.");
    }

    /**
     * Verifica si la navegación ha excedido el timeout.
     */
    pub fn is_navigation_timeout(&self) -> bool {
        if !self.is_navigating {
            return false;
        }
        match self.last_navigation_time {
            Some(time) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now - time > self.navigation_timeout_secs
            }
            None => false,
        }
    }
}