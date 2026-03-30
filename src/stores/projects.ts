import { defineStore } from "pinia";
import {
  createProject as createProjectFromApi,
  deleteProject as deleteProjectFromApi,
  listProjects,
  updateProject as updateProjectFromApi,
  type ProjectRecord
} from "@/api/tauri";

export interface ProjectItem {
  id: string;
  name: string;
  description: string;
  status: "active" | "archived";
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
}

function normalizeProjectStatus(value: string): "active" | "archived" {
  return value === "archived" ? "archived" : "active";
}

function normalizeTime(value: string) {
  if (/^\d+$/.test(value)) {
    return Number(value);
  }

  return Number(new Date(value)) || 0;
}

function mapProject(record: ProjectRecord): ProjectItem {
  return {
    id: record.id,
    name: record.name,
    description: record.description,
    status: normalizeProjectStatus(record.status),
    pinned: record.pinned,
    createdAt: record.created_at,
    updatedAt: record.updated_at
  };
}

function sortProjects(projects: ProjectItem[]) {
  return [...projects].sort((left, right) => {
    if (left.pinned !== right.pinned) {
      return left.pinned ? -1 : 1;
    }

    return normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt);
  });
}

export const useProjectsStore = defineStore("projects", {
  state: () => ({
    projects: [] as ProjectItem[],
    activeProjectId: "" as string,
    loaded: false,
    isBootstrapping: false,
    isSaving: false,
    error: "" as string
  }),
  getters: {
    activeProject(state) {
      return state.projects.find((project) => project.id === state.activeProjectId) ?? null;
    },
    pinnedCount(state) {
      return state.projects.filter((project) => project.pinned).length;
    },
    activeCount(state) {
      return state.projects.filter((project) => project.status === "active").length;
    },
    archivedCount(state) {
      return state.projects.filter((project) => project.status === "archived").length;
    }
  },
  actions: {
    setActiveProject(projectId: string) {
      this.activeProjectId = projectId;
    },
    clearActiveProject() {
      this.activeProjectId = "";
    },
    async bootstrap() {
      if (this.loaded || this.isBootstrapping) {
        return this.projects;
      }

      this.isBootstrapping = true;
      this.error = "";

      try {
        const records = await listProjects();
        this.projects = sortProjects(records.map(mapProject));
        this.loaded = true;
        if (!this.activeProjectId && this.projects.length > 0) {
          this.activeProjectId = this.projects[0].id;
        }
        return this.projects;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isBootstrapping = false;
      }
    },
    async createProject(input: {
      name: string;
      description: string;
      status: "active" | "archived";
      pinned: boolean;
    }) {
      this.isSaving = true;
      this.error = "";

      try {
        const record = await createProjectFromApi(
          input.name,
          input.description,
          input.status,
          input.pinned
        );
        const project = mapProject(record);
        this.projects = sortProjects([project, ...this.projects]);
        this.activeProjectId = project.id;
        this.loaded = true;
        return project;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async updateProject(projectId: string, input: {
      name: string;
      description: string;
      status: "active" | "archived";
      pinned: boolean;
    }) {
      this.isSaving = true;
      this.error = "";

      try {
        const record = await updateProjectFromApi(
          projectId,
          input.name,
          input.description,
          input.status,
          input.pinned
        );
        const project = mapProject(record);
        this.projects = sortProjects(
          this.projects.map((item) => (item.id === projectId ? project : item))
        );
        this.activeProjectId = project.id;
        return project;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async deleteProject(projectId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        await deleteProjectFromApi(projectId);
        this.projects = this.projects.filter((project) => project.id !== projectId);
        if (this.activeProjectId === projectId) {
          this.activeProjectId = this.projects[0]?.id ?? "";
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async togglePinned(projectId: string) {
      const project = this.projects.find((item) => item.id === projectId);
      if (!project) {
        return null;
      }

      return this.updateProject(projectId, {
        name: project.name,
        description: project.description,
        status: project.status,
        pinned: !project.pinned
      });
    },
    async setProjectStatus(projectId: string, status: "active" | "archived") {
      const project = this.projects.find((item) => item.id === projectId);
      if (!project) {
        return null;
      }

      return this.updateProject(projectId, {
        name: project.name,
        description: project.description,
        status,
        pinned: project.pinned
      });
    }
  }
});
