import { defineStore } from "pinia";
import {
  createCronJob,
  deleteCronJob,
  listCronJobs,
  listCronRuns,
  runCronJobNow,
  updateCronJob,
  type CronJobRecord,
  type CronRunRecord
} from "@/api/tauri";

export interface CronJobItem {
  id: string;
  name: string;
  schedule: string;
  prompt: string;
  enabled: boolean;
  lastRunAt: string | null;
  nextRunAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CronRunItem {
  id: string;
  jobId: string;
  status: string;
  output: string;
  startedAt: string;
  finishedAt: string;
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

function mapJob(record: CronJobRecord): CronJobItem {
  return {
    id: record.id,
    name: record.name,
    schedule: record.schedule,
    prompt: record.prompt,
    enabled: record.enabled,
    lastRunAt: record.last_run_at,
    nextRunAt: record.next_run_at,
    createdAt: record.created_at,
    updatedAt: record.updated_at
  };
}

function mapRun(record: CronRunRecord): CronRunItem {
  return {
    id: record.id,
    jobId: record.job_id,
    status: record.status,
    output: record.output,
    startedAt: record.started_at,
    finishedAt: record.finished_at
  };
}

function sortJobs(jobs: CronJobItem[]) {
  return [...jobs].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }

    return normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt);
  });
}

function sortRuns(runs: CronRunItem[]) {
  return [...runs].sort((left, right) => normalizeTime(right.startedAt) - normalizeTime(left.startedAt));
}

function upsertJob(jobs: CronJobItem[], job: CronJobItem) {
  const existing = jobs.some((item) => item.id === job.id);
  return sortJobs(existing ? jobs.map((item) => (item.id === job.id ? job : item)) : [job, ...jobs]);
}

function upsertRun(runs: CronRunItem[], run: CronRunItem) {
  return sortRuns([run, ...runs.filter((item) => item.id !== run.id)]);
}

export const useCronStore = defineStore("cron", {
  state: () => ({
    jobs: [] as CronJobItem[],
    selectedJobId: "" as string,
    runsByJobId: {} as Record<string, CronRunItem[]>,
    loaded: false,
    isLoading: false,
    isSaving: false,
    isRunning: false,
    isRunningJobId: "" as string,
    error: "" as string
  }),
  getters: {
    activeJob(state) {
      return state.jobs.find((job) => job.id === state.selectedJobId) ?? null;
    },
    activeRuns(state) {
      return state.runsByJobId[state.selectedJobId] ?? [];
    },
    enabledCount(state) {
      return state.jobs.filter((job) => job.enabled).length;
    },
    disabledCount(state) {
      return state.jobs.filter((job) => !job.enabled).length;
    }
  },
  actions: {
    setActiveJob(jobId: string) {
      this.selectedJobId = jobId;
    },
    applyJobRecord(record: CronJobRecord) {
      const job = mapJob(record);
      this.jobs = upsertJob(this.jobs, job);
      if (!this.selectedJobId) {
        this.selectedJobId = job.id;
      }
      return job;
    },
    applyRunRecord(record: CronRunRecord) {
      const run = mapRun(record);
      const currentRuns = this.runsByJobId[run.jobId] ?? [];
      this.runsByJobId[run.jobId] = upsertRun(currentRuns, run);
      return run;
    },
    async bootstrap() {
      if (this.loaded || this.isLoading) {
        return this.jobs;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const records = await listCronJobs();
        this.jobs = sortJobs(records.map(mapJob));
        this.loaded = true;
        if (!this.selectedJobId && this.jobs.length > 0) {
          this.selectedJobId = this.jobs[0].id;
        }
        if (this.selectedJobId) {
          await this.loadRuns(this.selectedJobId);
        }
        return this.jobs;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async loadRuns(jobId: string) {
      const runs = await listCronRuns(jobId);
      this.runsByJobId[jobId] = sortRuns(runs.map(mapRun));
      return this.runsByJobId[jobId];
    },
    async refreshJobs() {
      const records = await listCronJobs();
      this.jobs = sortJobs(records.map(mapJob));
      this.loaded = true;
      if (!this.selectedJobId && this.jobs.length > 0) {
        this.selectedJobId = this.jobs[0].id;
      }
      return this.jobs;
    },
    async createJob(input: { name: string; schedule: string; prompt: string; enabled: boolean }) {
      this.isSaving = true;
      this.error = "";

      try {
        const job = this.applyJobRecord(await createCronJob(input.name, input.schedule, input.prompt, input.enabled));
        this.selectedJobId = job.id;
        this.runsByJobId[job.id] = [];
        this.loaded = true;
        return job;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async updateJob(jobId: string, input: { name: string; schedule: string; prompt: string; enabled: boolean }) {
      this.isSaving = true;
      this.error = "";

      try {
        const job = this.applyJobRecord(await updateCronJob(jobId, input.name, input.schedule, input.prompt, input.enabled));
        this.selectedJobId = job.id;
        return job;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async deleteJob(jobId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        await deleteCronJob(jobId);
        this.jobs = this.jobs.filter((job) => job.id !== jobId);
        delete this.runsByJobId[jobId];
        if (this.selectedJobId === jobId) {
          this.selectedJobId = this.jobs[0]?.id ?? "";
          if (this.selectedJobId) {
            await this.loadRuns(this.selectedJobId);
          }
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async runNow(jobId: string) {
      this.isRunning = true;
      this.isRunningJobId = jobId;
      this.error = "";

      try {
        const run = this.applyRunRecord(await runCronJobNow(jobId));
        await this.refreshJobs();
        return run;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isRunning = false;
        this.isRunningJobId = "";
      }
    }
  }
});
