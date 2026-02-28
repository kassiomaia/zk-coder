// Shared Types - Frontend (TypeScript)
// These should match the Rust types in the backend

// ============ Project Types ============

export interface Project {
  id: string;
  name: string;
  path: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateProjectInput {
  name: string;
  path: string;
}

// ============ Settings Types ============

export interface Settings {
  theme: 'light' | 'dark' | 'system';
  autoSave: boolean;
  editorFont: string;
  fontSize: number;
  tabSize: number;
}

// ============ Error Types ============

export interface CommandError {
  code: ErrorCode;
  message: string;
  field?: string;
}

export type ErrorCode =
  | 'NOT_FOUND'
  | 'PERMISSION_DENIED'
  | 'INVALID_INPUT'
  | 'INTERNAL_ERROR';

// ============ Event Payloads ============

export interface ProjectCreatedPayload {
  project: Project;
}

export interface ProjectDeletedPayload {
  id: string;
}

export interface FileChangedPayload {
  path: string;
  content: string;
}

export interface NotificationPayload {
  type: 'info' | 'warning' | 'error';
  title: string;
  message: string;
}
