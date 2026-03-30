import { createRouter, createWebHashHistory } from "vue-router";
import ChannelsView from "@/views/ChannelsView.vue";
import ChatView from "@/views/ChatView.vue";
import McpView from "@/views/McpView.vue";
import ProjectsView from "@/views/ProjectsView.vue";
import SkillsView from "@/views/SkillsView.vue";
import CronView from "@/views/CronView.vue";
import SettingsView from "@/views/SettingsView.vue";

export const routes = [
  {
    path: "/",
    redirect: "/chat"
  },
  {
    path: "/chat",
    name: "chat",
    component: ChatView,
    meta: {
      titleKey: "routes.chat.title",
      descriptionKey: "routes.chat.description"
    }
  },
  {
    path: "/projects",
    name: "projects",
    component: ProjectsView,
    meta: {
      titleKey: "routes.projects.title",
      descriptionKey: "routes.projects.description"
    }
  },
  {
    path: "/channels",
    name: "channels",
    component: ChannelsView,
    meta: {
      titleKey: "routes.channels.title",
      descriptionKey: "routes.channels.description"
    }
  },
  {
    path: "/mcp",
    name: "mcp",
    component: McpView,
    meta: {
      titleKey: "routes.mcp.title",
      descriptionKey: "routes.mcp.description"
    }
  },
  {
    path: "/skills",
    name: "skills",
    component: SkillsView,
    meta: {
      titleKey: "routes.skills.title",
      descriptionKey: "routes.skills.description"
    }
  },
  {
    path: "/cron",
    name: "cron",
    component: CronView,
    meta: {
      titleKey: "routes.cron.title",
      descriptionKey: "routes.cron.description"
    }
  },
  {
    path: "/settings",
    name: "settings",
    component: SettingsView,
    meta: {
      titleKey: "routes.settings.title",
      descriptionKey: "routes.settings.description"
    }
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: "/chat"
  }
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes
});
