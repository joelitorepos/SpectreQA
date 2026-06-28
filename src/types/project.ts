// src/types/project.ts

export type ProjectEnv = 'local' | 'production';

export interface Project {
  id: string;
  name: string;
  url: string;
  env: ProjectEnv;
  commands: string[];
  projectPath: string;
  auditing: boolean; // el usuario activa esto explícitamente
  createdAt: string;
}