// src-tauri/src/session.rs

use std::time::Instant;
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
    /** Marca de tiempo del último EXECUTE_PHASE enviado al cliente */
    pub last_phase_sent_at: Option<Instant>,
    // NUEVOS CAMPOS PARA CONTROL DE NAVEGACION
    pub is_navigating: bool,
    pub last_navigation_time: Option<u64>,
    pub navigation_timeout_secs: u64,
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
            navigation_timeout_secs: 10, // 10 segundos máximo para navegación
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