<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { onCronJobUpdated, onCronRunRecorded } from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { useCronStore, type CronJobItem } from "@/stores/cron";

const cronStore = useCronStore();
const { activeJob, activeRuns, disabledCount, enabledCount, jobs } = storeToRefs(cronStore);
const { t } = useI18n();

const filter = ref<"all" | "enabled" | "disabled">("all");
const search = ref("");
const feedback = ref("");

const form = reactive({
  name: "",
  schedule: "0 0 * * * * *",
  prompt: "",
  enabled: true
});

let disposeJobUpdated: (() => void) | null = null;
let disposeRunRecorded: (() => void) | null = null;

const filteredJobs = computed(() => {
  const query = search.value.trim().toLowerCase();

  return jobs.value.filter((job) => {
    const matchesFilter =
      filter.value === "all"
        ? true
        : filter.value === "enabled"
          ? job.enabled
          : !job.enabled;

    if (!matchesFilter) {
      return false;
    }

    if (!query) {
      return true;
    }

    const haystack = `${job.name} ${job.schedule} ${job.prompt}`.toLowerCase();
    return haystack.includes(query);
  });
});

watch(
  activeJob,
  async (job) => {
    if (!job) {
      resetForm(true);
      return;
    }

    form.name = job.name;
    form.schedule = job.schedule;
    form.prompt = job.prompt;
    form.enabled = job.enabled;
    await cronStore.loadRuns(job.id);
  },
  { immediate: true }
);

onMounted(async () => {
  [disposeJobUpdated, disposeRunRecorded] = await Promise.all([
    onCronJobUpdated((record) => {
      cronStore.applyJobRecord(record);
    }),
    onCronRunRecorded((record) => {
      cronStore.applyRunRecord(record);
    })
  ]);

  if (!cronStore.loaded) {
    try {
      await cronStore.bootstrap();
    } catch {
      // the store error is rendered in the page
    }
  }
});

onBeforeUnmount(() => {
  disposeJobUpdated?.();
  disposeRunRecorded?.();
  disposeJobUpdated = null;
  disposeRunRecorded = null;
});

function resetForm(clearSelection = false) {
  if (clearSelection || !activeJob.value) {
    if (clearSelection) {
      cronStore.setActiveJob("");
    }
    form.name = "";
    form.schedule = "0 0 * * * * *";
    form.prompt = "";
    form.enabled = true;
    return;
  }

  form.name = activeJob.value.name;
  form.schedule = activeJob.value.schedule;
  form.prompt = activeJob.value.prompt;
  form.enabled = activeJob.value.enabled;
}

function resolveStatusLabel(job: CronJobItem) {
  return job.enabled ? t("cron.enabled") : t("cron.paused");
}

async function handleSubmit() {
  feedback.value = "";

  try {
    if (activeJob.value) {
      await cronStore.updateJob(activeJob.value.id, { ...form });
      feedback.value = t("cron.feedback.updated", { name: form.name });
      return;
    }

    const job = await cronStore.createJob({ ...form });
    feedback.value = t("cron.feedback.created", { name: job.name });
  } catch {
    feedback.value = activeJob.value ? t("cron.feedback.updateFailed") : t("cron.feedback.createFailed");
  }
}

function handleNewJob() {
  resetForm(true);
  feedback.value = t("cron.feedback.readyForNewJob");
}

async function handleSelectJob(jobId: string) {
  cronStore.setActiveJob(jobId);
  feedback.value = "";

  try {
    await cronStore.loadRuns(jobId);
  } catch {
    feedback.value = t("cron.feedback.loadRunsFailed");
  }
}

async function handleDeleteJob(jobId: string) {
  const job = jobs.value.find((item) => item.id === jobId);
  if (!job) {
    return;
  }

  const confirmed = window.confirm(t("cron.prompts.deleteJob", { name: job.name }));
  if (!confirmed) {
    return;
  }

  try {
    await cronStore.deleteJob(jobId);
    feedback.value = t("cron.feedback.deleted", { name: job.name });
  } catch {
    feedback.value = t("cron.feedback.deleteFailed");
  }
}

async function handleToggleEnabled(job: CronJobItem) {
  try {
    const updated = await cronStore.updateJob(job.id, {
      name: job.name,
      schedule: job.schedule,
      prompt: job.prompt,
      enabled: !job.enabled
    });
    feedback.value = updated.enabled ? t("cron.feedback.enabled", { name: job.name }) : t("cron.feedback.paused", { name: job.name });
  } catch {
    feedback.value = t("cron.feedback.toggleFailed");
  }
}

async function handleRunNow(jobId: string) {
  const job = jobs.value.find((item) => item.id === jobId);
  if (!job) {
    return;
  }

  try {
    const run = await cronStore.runNow(jobId);
    feedback.value = run.status === "success"
      ? t("cron.feedback.ranNow", { name: job.name })
      : `${t("cron.feedback.runFailed")} ${run.output}`;
  } catch {
    feedback.value = t("cron.feedback.runFailed");
  }
}
</script>

<template>
  <div class="stack cron-page">
    <section class="panel cron-hero">
      <div class="stack" style="gap: 8px; max-width: 760px;">
        <strong>{{ t("cron.workspaceTitle") }}</strong>
        <p class="muted cron-hero__copy">{{ t("cron.workspaceDescription") }}</p>
      </div>
      <div class="cron-summary-grid">
        <button class="summary-card" type="button" :data-active="filter === 'all'" @click="filter = 'all'">
          <strong>{{ jobs.length }}</strong>
          <span class="muted">{{ t("cron.summaryAll") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'enabled'" @click="filter = 'enabled'">
          <strong>{{ enabledCount }}</strong>
          <span class="muted">{{ t("cron.summaryEnabled") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'disabled'" @click="filter = 'disabled'">
          <strong>{{ disabledCount }}</strong>
          <span class="muted">{{ t("cron.summaryDisabled") }}</span>
        </button>
        <div class="summary-card summary-card--static">
          <strong>{{ activeRuns.length }}</strong>
          <span class="muted">{{ t("cron.summaryRuns") }}</span>
        </div>
      </div>
    </section>

    <section class="cron-layout">
      <section class="panel cron-editor">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ activeJob ? t("cron.editing", { name: activeJob.name }) : t("cron.newJob") }}</strong>
            <span class="muted">{{ t("cron.editorDescription") }}</span>
          </div>
          <Button variant="secondary" :disabled="cronStore.isSaving || cronStore.isRunning" @click="handleNewJob">{{ t("cron.newDraft") }}</Button>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("cron.jobName") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('cron.jobNamePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("cron.schedule") }}</span>
          <input v-model="form.schedule" class="field" :placeholder="t('cron.schedulePlaceholder')" />
          <span class="muted settings-field__hint">{{ t("cron.scheduleHint") }}</span>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("cron.prompt") }}</span>
          <textarea v-model="form.prompt" class="field cron-textarea" :placeholder="t('cron.promptPlaceholder')" />
        </label>

        <label class="projects-checkbox">
          <input v-model="form.enabled" type="checkbox" />
          <span>{{ t("cron.enabledToggle") }}</span>
        </label>

        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px; margin-top: 8px;">
          <div class="stack" style="gap: 6px; max-width: 560px;">
            <span v-if="feedback" class="muted">{{ feedback }}</span>
            <span v-if="cronStore.error" class="settings-error">{{ cronStore.error }}</span>
          </div>
          <div class="row settings-action-row">
            <Button variant="secondary" :disabled="cronStore.isSaving || cronStore.isRunning" @click="resetForm()">{{ t("cron.resetForm") }}</Button>
            <Button :disabled="cronStore.isSaving || cronStore.isRunning" @click="handleSubmit">
              {{ cronStore.isSaving ? t("cron.saving") : activeJob ? t("cron.saveJob") : t("cron.createJob") }}
            </Button>
          </div>
        </div>
      </section>

      <section class="panel cron-board">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("cron.boardTitle") }}</strong>
            <span class="muted">{{ t("cron.boardDescription") }}</span>
          </div>
          <input v-model="search" class="field cron-search" :placeholder="t('cron.searchPlaceholder')" />
        </div>

        <div v-if="cronStore.isLoading" class="empty-state">
          <strong>{{ t("cron.loadingTitle") }}</strong>
          <span class="muted">{{ t("cron.loadingDescription") }}</span>
        </div>

        <div v-else-if="filteredJobs.length === 0" class="empty-state">
          <strong>{{ t("cron.emptyTitle") }}</strong>
          <span class="muted">{{ t("cron.emptyDescription") }}</span>
        </div>

        <div v-else class="cron-list">
          <article
            v-for="job in filteredJobs"
            :key="job.id"
            class="cron-card"
            :data-active="job.id === cronStore.selectedJobId"
            @click="handleSelectJob(job.id)"
          >
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px;">
              <div class="stack" style="gap: 6px; min-width: 0;">
                <div class="row" style="align-items: center; gap: 8px; flex-wrap: wrap;">
                  <strong>{{ job.name }}</strong>
                  <span class="project-badge" :data-archived="!job.enabled">{{ resolveStatusLabel(job) }}</span>
                </div>
                <p class="cron-card__schedule">{{ job.schedule }}</p>
                <p class="cron-card__prompt">{{ job.prompt }}</p>
              </div>
            </div>

            <div class="row cron-card__meta">
              <span class="muted">{{ t("cron.nextRun", { value: job.nextRunAt ? formatTimestamp(job.nextRunAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) : t('cron.none') }) }}</span>
              <span class="muted">{{ t("cron.lastRun", { value: job.lastRunAt ? formatTimestamp(job.lastRunAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) : t('cron.none') }) }}</span>
            </div>

            <div class="row settings-action-row" style="justify-content: flex-end;">
              <Button variant="ghost" :disabled="cronStore.isSaving || cronStore.isRunning" @click.stop="handleToggleEnabled(job)">
                {{ job.enabled ? t("cron.pause") : t("cron.enable") }}
              </Button>
              <Button variant="secondary" :disabled="cronStore.isSaving || cronStore.isRunning" @click.stop="handleRunNow(job.id)">
                {{ cronStore.isRunning && cronStore.isRunningJobId === job.id ? t("cron.running") : t("cron.runNow") }}
              </Button>
              <Button variant="ghost" :disabled="cronStore.isSaving || cronStore.isRunning" @click.stop="handleDeleteJob(job.id)">
                {{ t("cron.delete") }}
              </Button>
            </div>
          </article>
        </div>
      </section>
    </section>

    <section class="panel cron-history-panel">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
        <div class="stack" style="gap: 4px;">
          <strong>{{ activeJob ? t("cron.historyTitle", { name: activeJob.name }) : t("cron.historyFallback") }}</strong>
          <span class="muted">{{ t("cron.historyDescription") }}</span>
        </div>
        <span v-if="activeJob" class="project-badge">{{ t("cron.summaryRuns") }}: {{ activeRuns.length }}</span>
      </div>

      <div v-if="!activeJob" class="empty-state">
        <strong>{{ t("cron.selectJobTitle") }}</strong>
        <span class="muted">{{ t("cron.selectJobDescription") }}</span>
      </div>

      <div v-else-if="activeRuns.length === 0" class="empty-state">
        <strong>{{ t("cron.emptyRunsTitle") }}</strong>
        <span class="muted">{{ t("cron.emptyRunsDescription") }}</span>
      </div>

      <div v-else class="cron-run-list">
        <article v-for="run in activeRuns" :key="run.id" class="cron-run-card">
          <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
            <div class="stack" style="gap: 4px; max-width: 820px;">
              <strong>{{ run.status }}</strong>
              <span class="muted">{{ t("cron.startedAt", { value: formatTimestamp(run.startedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              <span class="muted">{{ t("cron.finishedAt", { value: formatTimestamp(run.finishedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
            </div>
          </div>
          <div class="code-block">{{ run.output }}</div>
        </article>
      </div>
    </section>
  </div>
</template>
