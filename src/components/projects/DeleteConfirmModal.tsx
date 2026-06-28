// src/components/projects/DeleteConfirmModal.tsx
import type { Project } from '../../types/project';

interface DeleteConfirmModalProps {
  project: Project | null;
  onClose: () => void;
  onConfirm: () => void;
}

const DeleteConfirmModal = ({ project, onClose, onConfirm }: DeleteConfirmModalProps) => {
  if (!project) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-xl border border-slate-200 w-full max-w-sm mx-4 p-6 flex flex-col gap-4">

        {/* Icono */}
        <div className="w-10 h-10 rounded-xl bg-red-50 flex items-center justify-center">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#ef4444" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/>
            <path d="M10 11v6m4-6v6"/><path d="M9 6V4h6v2"/>
          </svg>
        </div>

        {/* Text */}
        <div>
          <h3 className="text-sm font-semibold text-slate-800">Eliminar proyecto</h3>
          <p className="text-sm text-slate-500 mt-1">
            ¿Seguro que quieres eliminar{' '}
            <span className="font-medium text-slate-700">"{project.name}"</span>?
            Esta acción no se puede deshacer.
          </p>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-1">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-xl border border-slate-200 text-slate-500 hover:bg-slate-50 transition"
          >
            Cancelar
          </button>
          <button
            onClick={onConfirm}
            className="px-4 py-2 text-sm font-medium rounded-xl bg-red-500 text-white hover:bg-red-600 transition"
          >
            Eliminar
          </button>
        </div>

      </div>
    </div>
  );
};

export default DeleteConfirmModal;