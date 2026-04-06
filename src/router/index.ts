import { createRouter, createWebHashHistory } from "vue-router";
import ChannelsView from "@/views/ChannelsView.vue";
import ChatView from "@/views/ChatView.vue";
import McpView from "@/views/McpView.vue";
import ProjectsView from "@/views/ProjectsView.vue";
import SkillsView from "@/views/SkillsView.vue";
import CronView from "@/views/CronView.vue";
import SettingsView from "@/views/SettingsView.vue";
import ChannelCreateView from "@/views/ChannelCreateView.vue";
import McpCreateView from "@/views/McpCreateView.vue";

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
    path: "/channels/new",
    name: "channels-create",
    component: ChannelCreateView,
    meta: {
      titleKey: "routes.channelsCreate.title",
      descriptionKey: "routes.channelsCreate.description"
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
    path: "/mcp/new",
    name: "mcp-create",
    component: McpCreateView,
    meta: {
      titleKey: "routes.mcpCreate.title",
      descriptionKey: "routes.mcpCreate.description"
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
    path: "/skills/new",
    name: "skills-create",
    redirect: "/skills"
  },
  {
    path: "/skills/:skillId/edit",
    name: "skills-edit",
    redirect: "/skills"
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
