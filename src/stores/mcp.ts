import { defineStore } from "pinia";
import {
  createMcpServer,
  deleteMcpServer,
  discoverMcpServerTools,
  listMcpServers,
  testMcpServer,
  updateMcpServer,
  type McpServerDraft,
  type McpServerRecord,
  type McpServerTestReport,
  type McpServerTestResult,
  type McpServerToolsResult,
  type McpToolRecord
} from "@/api/tauri";

export interface McpServerItem {
  id: string;
  name: string;
  transport: string;
  command: string;
  argumentsJson: string;
  url: string;
  headersJson: string;
  environmentJson: string;
  enabled: boolean;
  lastTestedAt: string | null;
  lastTestStatus: string | null;
  lastTestMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface McpServerTestItem {
  ok: boolean;
  transport: string;
  message: string;
  details: string | null;
  checkedAt: string;
}

export interface McpToolItem {
  fullName: string;
  toolName: string;
  serverName: string;
  description: string;
  inputSchemaJson: string;
}

export interface McpServerToolsItem {
  server: McpServerItem;
  tools: McpToolItem[];
  discoveredAt: string;
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

function mapServer(record: McpServerRecord): McpServerItem {
  return {
    id: record.id,
    name: record.name,
    transport: record.transport,
    command: record.command,
    argumentsJson: record.arguments_json,
    url: record.url,
    headersJson: record.headers_json,
    environmentJson: record.environment_json,
    enabled: record.enabled,
    lastTestedAt: record.last_tested_at,
    lastTestStatus: record.last_test_status,
    lastTestMessage: record.last_test_message,
    createdAt: record.created_at,
    updatedAt: record.updated_at
  };
}

function mapTestReport(report: McpServerTestReport): McpServerTestItem {
  return {
    ok: report.ok,
    transport: report.transport,
    message: report.message,
    details: report.details,
    checkedAt: report.checked_at
  };
}

function mapTool(record: McpToolRecord): McpToolItem {
  return {
    fullName: record.full_name,
    toolName: record.tool_name,
    serverName: record.server_name,
    description: record.description,
    inputSchemaJson: record.input_schema_json
  };
}

function mapToolDiscovery(result: McpServerToolsResult): McpServerToolsItem {
  return {
    server: mapServer(result.server),
    tools: result.tools.map(mapTool),
    discoveredAt: result.discovered_at
  };
}

function sortServers(servers: McpServerItem[]) {
  return [...servers].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }

    return normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt);
  });
}

function upsertServer(servers: McpServerItem[], server: McpServerItem) {
  const exists = servers.some((item) => item.id === server.id);
  return sortServers(exists ? servers.map((item) => (item.id === server.id ? server : item)) : [server, ...servers]);
}

function toDraft(input: McpServerItem | McpServerDraft): McpServerDraft {
  return {
    name: input.name,
    transport: input.transport,
    command: input.command,
    arguments_json: "argumentsJson" in input ? input.argumentsJson : input.arguments_json,
    url: input.url,
    headers_json: "headersJson" in input ? input.headersJson : input.headers_json,
    environment_json: "environmentJson" in input ? input.environmentJson : input.environment_json,
    enabled: input.enabled
  };
}

export const useMcpStore = defineStore("mcp", {
  state: () => ({
    servers: [] as McpServerItem[],
    activeServerId: "" as string,
    loaded: false,
    isLoading: false,
    isSaving: false,
    isTesting: false,
    isDiscovering: false,
    error: "" as string,
    lastTestReport: null as McpServerTestItem | null,
    lastToolDiscovery: null as McpServerToolsItem | null
  }),
  getters: {
    activeServer(state) {
      return state.servers.find((server) => server.id === state.activeServerId) ?? null;
    },
    enabledCount(state) {
      return state.servers.filter((server) => server.enabled).length;
    },
    stdioCount(state) {
      return state.servers.filter((server) => server.transport === "stdio").length;
    },
    remoteCount(state) {
      return state.servers.filter((server) => server.transport !== "stdio").length;
    },
    activeToolCount(state) {
      return state.lastToolDiscovery?.server.id === state.activeServerId ? state.lastToolDiscovery.tools.length : 0;
    }
  },
  actions: {
    setActiveServer(serverId: string) {
      this.activeServerId = serverId;
      this.clearDiscovery();
    },
    clearActiveServer() {
      this.activeServerId = "";
      this.lastTestReport = null;
      this.clearDiscovery();
    },
    clearDiscovery() {
      this.lastToolDiscovery = null;
    },
    applyServer(record: McpServerRecord) {
      const server = mapServer(record);
      this.servers = upsertServer(this.servers, server);
      if (!this.activeServerId) {
        this.activeServerId = server.id;
      }
      return server;
    },
    applyToolDiscovery(result: McpServerToolsResult) {
      const discovery = mapToolDiscovery(result);
      this.applyServer(result.server);
      this.lastToolDiscovery = discovery;
      return discovery;
    },
    async bootstrap() {
      if (this.loaded || this.isLoading) {
        return this.servers;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const records = await listMcpServers();
        this.servers = sortServers(records.map(mapServer));
        this.loaded = true;
        if (!this.activeServerId && this.servers.length > 0) {
          this.activeServerId = this.servers[0].id;
        }
        return this.servers;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async createServer(server: McpServerDraft) {
      this.isSaving = true;
      this.error = "";

      try {
        const created = this.applyServer(await createMcpServer(server));
        this.activeServerId = created.id;
        this.loaded = true;
        this.clearDiscovery();
        return created;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async updateServer(serverId: string, server: McpServerDraft) {
      this.isSaving = true;
      this.error = "";

      try {
        const updated = this.applyServer(await updateMcpServer(serverId, server));
        this.activeServerId = updated.id;
        this.clearDiscovery();
        return updated;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async deleteServer(serverId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        await deleteMcpServer(serverId);
        this.servers = this.servers.filter((server) => server.id !== serverId);
        if (this.activeServerId === serverId) {
          this.activeServerId = this.servers[0]?.id ?? "";
          this.lastTestReport = null;
          this.clearDiscovery();
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async testServer(serverId: string) {
      this.isTesting = true;
      this.error = "";

      try {
        const result = await testMcpServer(serverId);
        this.applyServer(result.server);
        this.lastTestReport = mapTestReport(result.report);
        return result;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isTesting = false;
      }
    },
    async discoverServerTools(serverId: string) {
      this.isDiscovering = true;
      this.error = "";

      try {
        const result = await discoverMcpServerTools(serverId);
        return this.applyToolDiscovery(result);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isDiscovering = false;
      }
    },
    makeDraftFromActive() {
      return this.activeServer ? toDraft(this.activeServer) : null;
    },
    resetLastTest(result: McpServerTestResult | null = null) {
      this.lastTestReport = result ? mapTestReport(result.report) : null;
    }
  }
});
