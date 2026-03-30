import { defineStore } from "pinia";
import {
  createChannel,
  deleteChannel,
  getChannelRuntimeStatus,
  listChannels,
  startChannelRuntime,
  stopChannelRuntime,
  testChannel,
  updateChannel,
  type ChannelDraft,
  type ChannelRecord,
  type ChannelRuntimeStatusRecord,
  type ChannelTestReport,
  type ChannelTestResult
} from "@/api/tauri";

export interface ChannelItem {
  id: string;
  name: string;
  kind: string;
  configJson: string;
  enabled: boolean;
  lastCheckedAt: string | null;
  lastHealthStatus: string | null;
  lastHealthMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ChannelTestItem {
  ok: boolean;
  kind: string;
  message: string;
  checkedAt: string;
}

export interface ChannelRuntimeStatusItem {
  running: boolean;
  state: string;
  message: string;
  updatedAt: string;
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

function mapChannel(record: ChannelRecord): ChannelItem {
  return {
    id: record.id,
    name: record.name,
    kind: record.kind,
    configJson: record.config_json,
    enabled: record.enabled,
    lastCheckedAt: record.last_checked_at,
    lastHealthStatus: record.last_health_status,
    lastHealthMessage: record.last_health_message,
    createdAt: record.created_at,
    updatedAt: record.updated_at
  };
}

function mapTestReport(report: ChannelTestReport): ChannelTestItem {
  return {
    ok: report.ok,
    kind: report.kind,
    message: report.message,
    checkedAt: report.checked_at
  };
}

function mapRuntimeStatus(status: ChannelRuntimeStatusRecord): ChannelRuntimeStatusItem {
  return {
    running: status.running,
    state: status.state,
    message: status.message,
    updatedAt: status.updated_at
  };
}

function sortChannels(channels: ChannelItem[]) {
  return [...channels].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }

    return normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt);
  });
}

function upsertChannel(channels: ChannelItem[], channel: ChannelItem) {
  const exists = channels.some((item) => item.id === channel.id);
  return sortChannels(exists ? channels.map((item) => (item.id === channel.id ? channel : item)) : [channel, ...channels]);
}

export const defaultRuntimeStatus = (): ChannelRuntimeStatusItem => ({
  running: false,
  state: "idle",
  message: "Channels supervisor is idle.",
  updatedAt: ""
});

export const useChannelStore = defineStore("channels", {
  state: () => ({
    channels: [] as ChannelItem[],
    activeChannelId: "" as string,
    loaded: false,
    isLoading: false,
    isSaving: false,
    isTesting: false,
    isStartingRuntime: false,
    isStoppingRuntime: false,
    error: "" as string,
    runtimeStatus: defaultRuntimeStatus() as ChannelRuntimeStatusItem,
    lastTestReport: null as ChannelTestItem | null
  }),
  getters: {
    activeChannel(state) {
      return state.channels.find((channel) => channel.id === state.activeChannelId) ?? null;
    },
    enabledCount(state) {
      return state.channels.filter((channel) => channel.enabled).length;
    },
    healthyCount(state) {
      return state.channels.filter((channel) => channel.lastHealthStatus === "healthy").length;
    }
  },
  actions: {
    setActiveChannel(channelId: string) {
      this.activeChannelId = channelId;
    },
    clearActiveChannel() {
      this.activeChannelId = "";
      this.lastTestReport = null;
    },
    applyChannel(record: ChannelRecord) {
      const channel = mapChannel(record);
      this.channels = upsertChannel(this.channels, channel);
      if (!this.activeChannelId) {
        this.activeChannelId = channel.id;
      }
      return channel;
    },
    applyRuntimeStatus(status: ChannelRuntimeStatusRecord | ChannelRuntimeStatusItem) {
      this.runtimeStatus = "updated_at" in status ? mapRuntimeStatus(status) : status;
      return this.runtimeStatus;
    },
    async bootstrap() {
      if (this.loaded || this.isLoading) {
        return this.channels;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const [channels, runtimeStatus] = await Promise.all([listChannels(), getChannelRuntimeStatus()]);
        this.channels = sortChannels(channels.map(mapChannel));
        this.runtimeStatus = mapRuntimeStatus(runtimeStatus);
        this.loaded = true;
        if (!this.activeChannelId && this.channels.length > 0) {
          this.activeChannelId = this.channels[0].id;
        }
        return this.channels;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async createChannel(channel: ChannelDraft) {
      this.isSaving = true;
      this.error = "";

      try {
        const created = this.applyChannel(await createChannel(channel));
        this.activeChannelId = created.id;
        this.loaded = true;
        return created;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async updateChannel(channelId: string, channel: ChannelDraft) {
      this.isSaving = true;
      this.error = "";

      try {
        const updated = this.applyChannel(await updateChannel(channelId, channel));
        this.activeChannelId = updated.id;
        return updated;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async deleteChannel(channelId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        await deleteChannel(channelId);
        this.channels = this.channels.filter((channel) => channel.id !== channelId);
        if (this.activeChannelId === channelId) {
          this.activeChannelId = this.channels[0]?.id ?? "";
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async testChannel(channelId: string) {
      this.isTesting = true;
      this.error = "";
      this.lastTestReport = null;

      try {
        const result = await testChannel(channelId);
        this.applyChannel(result.channel);
        this.lastTestReport = mapTestReport(result.report);
        return result as ChannelTestResult;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isTesting = false;
      }
    },
    async startRuntime() {
      this.isStartingRuntime = true;
      this.error = "";

      try {
        this.runtimeStatus = mapRuntimeStatus(await startChannelRuntime());
        return this.runtimeStatus;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isStartingRuntime = false;
      }
    },
    async stopRuntime() {
      this.isStoppingRuntime = true;
      this.error = "";

      try {
        this.runtimeStatus = mapRuntimeStatus(await stopChannelRuntime());
        return this.runtimeStatus;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isStoppingRuntime = false;
      }
    }
  }
});
