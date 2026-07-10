// Skills system types (engineering-todo-v2.md 1.9.1).

export type SkillScope = "global" | "project" | "nested";

export interface SkillMetadata {
  id: string;
  name: string;
  description: string;
  scope: SkillScope;
  directory_path: string;
  file_path: string;
  frontmatter: Record<string, unknown>;
  body: string;
  is_builtin: boolean;
  is_active: boolean;
  when_to_use: string | null;
  created_at: number;
  updated_at: number;
}

/** Lightweight catalog entry sent to agents - name + description only. */
export interface SkillCatalogEntry {
  name: string;
  description: string;
  when_to_use: string | null;
}

export interface ScannedSkillCandidate {
  name: string;
  file_path: string;
  directory_path: string;
  preview: string;
}
