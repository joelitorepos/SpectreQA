// src/pages/DocsPage.tsx
import { useState } from 'react';

const CodeBlock = ({ code }: { code: string }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="relative group">
      <pre className="text-xs font-mono bg-slate-900 text-slate-100 rounded-xl p-4 overflow-x-auto leading-relaxed">
        {code}
      </pre>
      <button
        onClick={handleCopy}
        className="absolute top-2 right-2 text-[11px] px-2 py-1 rounded-lg bg-slate-700 text-slate-200 hover:bg-slate-600 transition opacity-0 group-hover:opacity-100"
      >
        {copied ? 'Copiado ✓' : 'Copiar'}
      </button>
    </div>
  );
};

const Section = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <div className="flex flex-col gap-2">
    <h2 className="text-sm font-semibold text-slate-800">{title}</h2>
    <div className="flex flex-col gap-2 text-sm text-slate-600 leading-relaxed">{children}</div>
  </div>
);

const DocsPage = () => {
  return (
    <div className="flex flex-col gap-8 max-w-2xl pb-12">
      <div>
        <h1 className="text-lg font-semibold text-slate-800">Documentación</h1>
        <p className="text-sm text-slate-400 mt-0.5">Lo básico para empezar a usar SpectreQA</p>
      </div>

      <Section title="1. Crea un proyecto">
        <p>
          En la página <strong>Proyectos</strong>, crea un nuevo proyecto. Cada proyecto tiene su
          propia tarjeta — ahí puedes editar el prompt (el archivo <code>AGENT.md</code>) que le
          dice a la IA qué probar, y modificar los datos del proyecto cuando lo necesites.
        </p>
      </Section>

      <Section title="2. Elige tu modelo de IA">
        <p>
          En <strong>Configuración</strong> decides qué IA usa el agente:
        </p>
        <ul className="list-disc list-inside space-y-1">
          <li>
            <strong>Ollama (local):</strong> instala{' '}
            <a
              href="https://ollama.com/download"
              target="_blank"
              rel="noreferrer"
              className="text-[#534AB7] underline underline-offset-2 hover:opacity-80"
            >
              Ollama
            </a>{' '}
            y descarga un modelo abierto (por ejemplo <code>llama3.2</code>) para correr todo en tu
            propia máquina, sin API key.
          </li>
          <li>
            <strong>SpectreQA Cloud:</strong> genera tu API key en el{' '}
            <a
              href="https://spectreqa.com/dashboard"
              target="_blank"
              rel="noreferrer"
              className="text-[#534AB7] underline underline-offset-2 hover:opacity-80"
            >
              dashboard de la página oficial
            </a>{' '}
            y pégala en Configuración, descubre nuestros planes para aumentar tu usabilidad.
          </li>
        </ul>
      </Section>

      <Section title="3. Revisa tus resultados">
        <p>
          Después de correr una prueba, ve a <strong>Logs</strong> para ver fase por fase qué hizo
          la IA, qué comandos ejecutó y, si algo falló, el mensaje exacto del error.
        </p>
      </Section>

      <div className="h-px bg-slate-200" />

      <div>
        <h1 className="text-lg font-semibold text-slate-800">⚠️ Eventos custom del ciclo de vida</h1>
        <p className="text-sm text-slate-500 mt-1 leading-relaxed">
          La extensión no puede saber por sí sola cuándo tu aplicación terminó de cargar algo
          asíncrono, cuándo diste por exitosa o fallida una prueba, ni cuándo el usuario debe
          esperar. Por eso <strong>tu propia página bajo prueba</strong> es quien emite estos
          eventos — la extensión solo escucha. Todos se disparan igual, cambiando el{' '}
          <code>status</code>:
        </p>
      </div>

      <Section title="Detener (pausar) la prueba">
        <p>
          Pausa la ejecución indefinidamente. Útil si tu página necesita que el usuario haga algo
          manual antes de seguir. El último comando en curso termina de ejecutarse antes de
          pausar.
        </p>
        <CodeBlock
          code={`window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'PAUSE',
    message: 'Esperando confirmación manual del usuario',
    timestamp: Date.now()
  }
}));`}
        />
      </Section>

      <Section title="Continuar (reanudar tras pausa)">
        <p>Reanuda una prueba que estaba en pausa manual.</p>
        <CodeBlock
          code={`window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'RESUME',
    message: 'El usuario confirmó, continuar',
    timestamp: Date.now()
  }
}));`}
        />
      </Section>

      <Section title="Terminar con éxito">
        <p>
          Marca la prueba como completada exitosamente ahora mismo, sin esperar a que la IA agote
          sus fases. Útil cuando tu página sabe con certeza que el objetivo se cumplió (por
          ejemplo, llegaste a la pantalla de confirmación de una compra).
        </p>
        <CodeBlock
          code={`window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'TEST_COMPLETE',
    message: 'Compra confirmada - prueba exitosa',
    timestamp: Date.now()
  }
}));`}
        />
      </Section>

      <Section title="Terminar con error">
        <p>
          Marca la prueba como fallida ahora mismo. Úsalo cuando tu página detecta una condición
          de error real (por ejemplo, un mensaje de "pago rechazado" que la IA no debería
          interpretar como progreso normal).
        </p>
        <CodeBlock
          code={`window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'ERROR',
    message: 'El pago fue rechazado',
    timestamp: Date.now()
  }
}));`}
        />
      </Section>

      <Section title="Esperar un evento asíncrono (WAIT)">
        <p>
          El caso más común: tu página necesita tiempo para responder a algo (una llamada a tu
          backend, una animación de carga, una redirección) antes de que la IA vuelva a mirar el
          DOM. Sin esto, la IA podría intentar interactuar con elementos que todavía no existen o
          que están a punto de desaparecer.
        </p>
        <CodeBlock
          code={`window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'WAIT',
    message: 'Esperando respuesta del backend',
    timestamp: Date.now()
  }
}));

// Cuando termine la espera real (tu propio código, tu propio callback):
// hay que "soltar" el WAIT explícitamente. WAIT no se resuelve solo —
// se resuelve dejando pasar la navegación (la extensión detecta el nuevo
// DOM) o, si no hubo navegación, emitiendo CONTINUE/RESUME cuando ya
// terminó lo que estabas esperando:
window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
  detail: {
    status: 'CONTINUE',
    message: 'Datos cargados, continuar',
    timestamp: Date.now()
  }
}));`}
        />
      </Section>

      <Section title="Detectar que la extensión ya se inyectó (tras cambiar de página)">
        <p>
          Cuando tu página navega (por ejemplo, tras un login exitoso), la extensión se reinyecta
          desde cero en el documento nuevo — pierde cualquier estado anterior por un instante. Si
          tu página dispara un <code>WAIT</code> inmediatamente al cargar, corre el riesgo de
          emitirlo <strong>antes</strong> de que la extensión esté escuchando, y ese evento se
          perdería para siempre. La solución: esperar a que la extensión avise que ya está lista.
        </p>
        <CodeBlock
          code={`(function () {
  let iniciado = false;

  function emitirEsperaSiCorresponde() {
    if (iniciado) return;
    iniciado = true;

    window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
      detail: {
        status: 'WAIT',
        message: 'Esperando datos del backend',
        timestamp: Date.now()
      }
    }));

    // Cuando tu carga real termine, resuelve el WAIT:
    // window.dispatchEvent(new CustomEvent('__spectreqa_lifecycle_event__', {
    //   detail: { status: 'CONTINUE', message: 'Listo', timestamp: Date.now() }
    // }));
  }

  // 1. REACTIVO: la extensión avisa que ya se inyectó y está escuchando.
  window.addEventListener('__spectreqa_extension_ready__', emitirEsperaSiCorresponde);

  // 2. VERIFICACIÓN DIRECTA: por si la extensión se inyectó milisegundos
  // antes de que este script llegara a ejecutarse (evita perder el evento
  // si la extensión fue más rápida que tu página).
  if (window.__spectreqa_lifecycle__ || window.__spectreqa_lifecycle_manager__) {
    emitirEsperaSiCorresponde();
  }
})();`}
        />
      </Section>

      <div className="bg-amber-50 border border-amber-200 rounded-xl px-4 py-3 text-xs text-amber-700 leading-relaxed">
        <strong>Importante:</strong> todo evento que pausa la ejecución (<code>PAUSE</code> o{' '}
        <code>WAIT</code>) debe resolverse eventualmente con <code>CONTINUE</code>/
        <code>RESUME</code>, o la prueba se queda esperando indefinidamente hasta que el watchdog
        de la extensión la corte por timeout.
      </div>
    </div>
  );
};

export default DocsPage;