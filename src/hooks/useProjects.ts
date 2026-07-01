// src/hooks/useProjects.ts
import { useEffect, useState } from 'react';
import { load, Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import type { Project } from '../types/project';

const STORE_KEY = 'projects';
let storeInstance: Store | null = null;

const getStore = async (): Promise<Store> => {
  if (!storeInstance) storeInstance = await load('projects.json');
  return storeInstance;
};

export const useProjects = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const init = async () => {
      const store = await getStore();
      const saved = await store.get<Project[]>(STORE_KEY);
      const loaded = saved ?? [];
      setProjects(loaded);
      setLoading(false);

      // Si ya había un proyecto activo guardado, sincronizarlo con Rust
      // para que START_TEST funcione sin que el usuario tenga que hacer toggle.
      const activeProject = loaded.find((p) => p.auditing);
      if (activeProject) {
        await invoke('set_active_project', { projectId: activeProject.id }).catch(console.error);
        // También guardar en session storage de la extensión para que
        // background.js lo incluya en el mensaje START_TEST
        await setExtensionActiveProject(activeProject.id);
      }
    };
    init();
  }, []);

  const persist = async (updated: Project[]) => {
    const store = await getStore();
    await store.set(STORE_KEY, updated);
    await store.save();
    setProjects(updated);
  };

  const addProject = async (project: Project) => {
    await invoke('create_agent_md', {
      projectId: project.id,
      projectName: project.name,
      projectUrl: project.url,
      projectEnv: project.env,
    });
    await persist([...projects, project]);
  };

  const updateProject = async (updated: Project) => {
    await persist(projects.map((p) => (p.id === updated.id ? updated : p)));
  };

  /**
   * Activa o desactiva la auditoría de un proyecto.
   * Solo un proyecto puede estar activo a la vez.
   * Notifica a Rust y a la extensión (via session storage) cuál es el activo.
   */
  const toggleAuditing = async (id: string) => {
    const target = projects.find((p) => p.id === id);
    if (!target) return;

    const willBeActive = !target.auditing;

    const updated = projects.map((p) => ({
      ...p,
      auditing: p.id === id ? willBeActive : false,
    }));

    await persist(updated);

    if (willBeActive) {
      await invoke('set_active_project', { projectId: id }).catch(console.error);
      await setExtensionActiveProject(id);
    } else {
      await invoke('clear_active_project').catch(console.error);
      await setExtensionActiveProject(null);
    }
  };

  const removeProject = async (id: string) => {
    const wasActive = projects.find((p) => p.id === id)?.auditing ?? false;
    await invoke('delete_project_dir', { projectId: id });
    if (wasActive) {
      await invoke('clear_active_project').catch(console.error);
      await setExtensionActiveProject(null);
    }
    await persist(projects.filter((p) => p.id !== id));
  };

  return { projects, loading, addProject, updateProject, toggleAuditing, removeProject };
};

/**
 * Guarda el project_id activo en chrome.storage.session via la extensión,
 * para que background.js lo incluya en el mensaje START_TEST a Rust.
 * Se comunica con el SW usando chrome.runtime.sendMessage.
 */
async function setExtensionActiveProject(projectId: string | null): Promise<void> {
  try {
    await invoke('send_to_extension', {
      messageType: 'SET_ACTIVE_PROJECT',
      payload: { project_id: projectId },
    });
  } catch (e) {
    console.error('[SpectreQA] Error enviando SET_ACTIVE_PROJECT a la extensión:', e);
  }
}