import { defineStore } from "pinia";
import {
  createSkill as createSkillRequest,
  deleteSkill,
  getSkillDetail,
  importSkillDirectory,
  installSkillTemplate,
  listSkillTemplates,
  listSkills,
  setSkillEnabled,
  type SkillDraft,
  type SkillDetailRecord,
  type SkillRecord,
  type SkillTemplateRecord
} from "@/api/tauri";

export interface SkillItem {
  id: string;
  slug: string;
  name: string;
  description: string;
  version: string;
  author: string;
  tagsJson: string;
  sourceKind: string;
  sourceLabel: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface SkillTemplateItem {
  templateId: string;
  slug: string;
  name: string;
  description: string;
  author: string;
  tagsJson: string;
}

export interface SkillDetailItem {
  skill: SkillItem;
  markdownContent: string;
}

function normalizeTime(value: string | null | undefined) {
  if (!value) {
    return 0;
  }

  if (/^\d+$/.test(value)) {
    return Number(value);
  }

  return Number(new Date(value)) || 0;
}

function mapSkill(record: SkillRecord): SkillItem {
  return {
    id: record.id,
    slug: record.slug,
    name: record.name,
    description: record.description,
    version: record.version,
    author: record.author,
    tagsJson: record.tags_json,
    sourceKind: record.source_kind,
    sourceLabel: record.source_label,
    enabled: record.enabled,
    createdAt: record.created_at,
    updatedAt: record.updated_at
  };
}

function mapTemplate(record: SkillTemplateRecord): SkillTemplateItem {
  return {
    templateId: record.template_id,
    slug: record.slug,
    name: record.name,
    description: record.description,
    author: record.author,
    tagsJson: record.tags_json
  };
}

function mapDetail(record: SkillDetailRecord): SkillDetailItem {
  return {
    skill: mapSkill(record.skill),
    markdownContent: record.markdown_content
  };
}

function sortSkills(skills: SkillItem[]) {
  return [...skills].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }

    return normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt);
  });
}

function upsertSkill(skills: SkillItem[], skill: SkillItem) {
  const exists = skills.some((item) => item.id === skill.id);
  return sortSkills(exists ? skills.map((item) => (item.id === skill.id ? skill : item)) : [skill, ...skills]);
}

export function parseTags(tagsJson: string) {
  try {
    const parsed = JSON.parse(tagsJson);
    return Array.isArray(parsed) ? parsed.filter((value) => typeof value === "string") : [];
  } catch {
    return [];
  }
}

export const useSkillStore = defineStore("skills", {
  state: () => ({
    templates: [] as SkillTemplateItem[],
    skills: [] as SkillItem[],
    activeSkillId: "" as string,
    activeSkillDetail: null as SkillDetailItem | null,
    detailSkillId: "" as string,
    loaded: false,
    isLoading: false,
    isSaving: false,
    isImporting: false,
    error: "" as string
  }),
  getters: {
    activeSkill(state) {
      return state.skills.find((skill) => skill.id === state.activeSkillId) ?? null;
    },
    enabledCount(state) {
      return state.skills.filter((skill) => skill.enabled).length;
    },
    importedCount(state) {
      return state.skills.filter((skill) => skill.sourceKind === "imported").length;
    }
  },
  actions: {
    setActiveSkill(skillId: string) {
      this.activeSkillId = skillId;
      if (this.detailSkillId !== skillId) {
        this.activeSkillDetail = null;
      }
    },
    applySkill(record: SkillRecord) {
      const skill = mapSkill(record);
      this.skills = upsertSkill(this.skills, skill);
      if (!this.activeSkillId) {
        this.activeSkillId = skill.id;
      }
      if (this.activeSkillDetail?.skill.id === skill.id) {
        this.activeSkillDetail = {
          ...this.activeSkillDetail,
          skill
        };
        this.detailSkillId = skill.id;
      }
      return skill;
    },
    async bootstrap() {
      if (this.loaded || this.isLoading) {
        return this.skills;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const [templates, skills] = await Promise.all([listSkillTemplates(), listSkills()]);
        this.templates = templates.map(mapTemplate);
        this.skills = sortSkills(skills.map(mapSkill));
        this.loaded = true;
        if (!this.activeSkillId && this.skills.length > 0) {
          this.activeSkillId = this.skills[0].id;
        }
        return this.skills;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async loadSkillDetail(skillId: string) {
      if (this.detailSkillId === skillId && this.activeSkillDetail) {
        return this.activeSkillDetail;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const detail = mapDetail(await getSkillDetail(skillId));
        this.activeSkillDetail = detail;
        this.detailSkillId = skillId;
        this.activeSkillId = skillId;
        return detail;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async createSkill(skill: SkillDraft) {
      this.isSaving = true;
      this.error = "";

      try {
        const created = this.applySkill(await createSkillRequest(skill));
        await this.loadSkillDetail(created.id);
        this.loaded = true;
        return created;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async installTemplate(templateId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        const skill = this.applySkill(await installSkillTemplate(templateId));
        await this.loadSkillDetail(skill.id);
        this.loaded = true;
        return skill;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async importDirectory() {
      this.isImporting = true;
      this.error = "";

      try {
        const record = await importSkillDirectory();
        if (!record) {
          return null;
        }

        const skill = this.applySkill(record);
        await this.loadSkillDetail(skill.id);
        this.loaded = true;
        return skill;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isImporting = false;
      }
    },
    async toggleSkill(skillId: string, enabled: boolean) {
      this.isSaving = true;
      this.error = "";

      try {
        const skill = this.applySkill(await setSkillEnabled(skillId, enabled));
        this.loaded = true;
        return skill;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async removeSkill(skillId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        const removed = mapSkill(await deleteSkill(skillId));
        this.skills = this.skills.filter((skill) => skill.id !== skillId);

        if (this.activeSkillId === skillId) {
          this.activeSkillId = this.skills[0]?.id ?? "";
        }

        if (this.detailSkillId === skillId) {
          this.detailSkillId = "";
          this.activeSkillDetail = null;
        }

        this.loaded = true;
        return removed;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    }
  }
});
