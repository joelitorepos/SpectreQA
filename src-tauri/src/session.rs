// src-tauri/src/session.rs

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
 * Estado posible de una sesión de prueba.
 */
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Running,
    Finished,
    Error,
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
    /** Contador de reintentos cuando el DOM no cambia. Máximo 2. */
    pub no_change_retries: u32,
    /** Última URL conocida para detectar navegación (login exitoso) */
    pub last_url: Option<String>,
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
            last_url: None, // ← Nuevo campo
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
     * Registra un intento fallido por DOM sin cambios.
     * Devuelve true si se superó el límite de reintentos (2).
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
}