// src/components/layout/Sidebar.tsx
import NavButton from './NavButton';
import { useExtension } from '../../hooks/useExtension';
import { useOllama } from '../../hooks/useOllama';

const Sidebar = () => {
  const { connected } = useExtension();
  const { status: ollamaStatus } = useOllama();

  return (
    <aside className="flex flex-col w-56 shrink-0 bg-slate-50 border-r border-slate-200 h-screen sticky top-0">
      {/* Logo */}
      <div className="px-5 pt-6 pb-5 border-b border-slate-200">
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-[#534AB7] flex items-center justify-center text-white text-sm">
            ⬡
          </div>
          <span className="font-semibold text-slate-800 text-[15px]">SpectreQA</span>
        </div>
      </div>

      {/* Nav */}
      <nav className="flex-1 px-3 py-4 space-y-0.5">
        <NavButton
          to=""
          label="Proyectos"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/>
              <rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
            </svg>
          }
        />
        <NavButton
          to="settings"
          label="Configuración"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3"/>
              <path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/>
            </svg>
          }
        />
        <NavButton
          to="logs"
          label="Logs"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M8 6h12" /><path d="M8 12h8" /><path d="M8 18h6" />
              <path d="M4 6h.01" /><path d="M4 12h.01" /><path d="M4 18h.01" />
            </svg>
          }
        />
      </nav>

      {/* Extension status */}
      <div className="px-3 pb-3">
        <div className="bg-white border border-slate-200 rounded-xl px-3.5 py-3 space-y-1.5">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-slate-700">Extensión</span>
            <span className={`flex items-center gap-1.5 text-xs font-medium ${connected ? 'text-emerald-600' : 'text-slate-400'}`}>
              <span className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-emerald-500' : 'bg-slate-300'}`} />
              {connected ? 'Conectada' : 'Sin conectar'}
            </span>
          </div>
          {!connected && (
            <p className="text-[11px] text-slate-400 leading-tight">
              Instala la extensión para comenzar a auditar
            </p>
          )}
        </div>
      </div>

      {/* Ollama status - Sugerido, no obligatorio (hay alternativa en la nube) */}
      {ollamaStatus === 'offline' && (
        <div className="px-3 pb-3">
          <div className="bg-amber-50 border border-amber-200 rounded-xl px-3.5 py-3 space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-amber-700">Ollama</span>
              <span className="text-xs font-semibold text-amber-600">Sugerido</span>
            </div>
            <p className="text-[11px] text-amber-700 leading-normal">
              Opcional:{' '}
              <a
                href="https://ollama.com/download"
                target="_blank"
                rel="noreferrer"
                className="underline underline-offset-2 font-bold hover:text-amber-900"
              >
                descarga Ollama
              </a>{' '}
              para correr la IA localmente, o usa el servicio en la nube cuando esté disponible.
            </p>
          </div>
        </div>
      )}
    </aside>
  );
};

export default Sidebar;