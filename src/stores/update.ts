import { defineStore } from "pinia";
import {
  checkAppUpdate,
  getUpdateSettings,
  installAppUpdate,
  saveUpdateSettings,
  type UpdateCheckReport,
  type UpdateInstallReport,
  type UpdateSettingsRecord
} from "@/api/tauri";

export const defaultUpdateSettings = (): UpdateSettingsRecord => ({
  enabled: false,
  auto_check: true,
  endpoints: [],
  pubkey: ""
});

function cloneUpdateSettings(settings: UpdateSettingsRecord): UpdateSettingsRecord {
  return {
    enabled: settings.enabled,
    auto_check: settings.auto_check,
    endpoints: [...settings.endpoints],
    pubkey: settings.pubkey
  };
}

export const useUpdateStore = defineStore("update", {
  state: () => ({
    settings: defaultUpdateSettings() as UpdateSettingsRecord,
    lastCheck: null as UpdateCheckReport | null,
    lastInstall: null as UpdateInstallReport | null,
    isLoading: false,
    isSaving: false,
    isChecking: false,
    isInstalling: false,
    loaded: false,
    autoCheckCompleted: false,
    error: "" as string
  }),
  actions: {
    async bootstrap() {
      if (this.loaded || this.isLoading) {
        if (this.loaded && !this.autoCheckCompleted && this.settings.enabled && this.settings.auto_check) {
          this.autoCheckCompleted = true;
          try {
            await this.checkForUpdates();
          } catch {
            // keep the error state for the settings page
          }
        }
        return this.settings;
      }

      this.isLoading = true;
      this.error = "";

      try {
        this.settings = cloneUpdateSettings(await getUpdateSettings());
        this.loaded = true;

        if (!this.autoCheckCompleted && this.settings.enabled && this.settings.auto_check) {
          this.autoCheckCompleted = true;
          try {
            await this.checkForUpdates();
          } catch {
            // keep the error state for the settings page
          }
        }

        return this.settings;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async saveSettings(settings: UpdateSettingsRecord) {
      this.isSaving = true;
      this.error = "";

      try {
        this.settings = cloneUpdateSettings(await saveUpdateSettings(settings));
        this.loaded = true;
        this.autoCheckCompleted = !this.settings.enabled || !this.settings.auto_check;
        return this.settings;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async checkForUpdates() {
      this.isChecking = true;
      this.error = "";

      try {
        this.lastCheck = await checkAppUpdate();
        this.autoCheckCompleted = true;
        return this.lastCheck;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isChecking = false;
      }
    },
    async installLatestUpdate() {
      this.isInstalling = true;
      this.error = "";

      try {
        this.lastInstall = await installAppUpdate();
        this.autoCheckCompleted = true;
        return this.lastInstall;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isInstalling = false;
      }
    }
  }
});
