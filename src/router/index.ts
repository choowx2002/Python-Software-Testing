import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "Dashboard",
      component: () => import("@/views/Dashboard.vue"),
    },
    {
      path: "/projects/import",
      name: "ProjectImport",
      component: () => import("@/views/projects/Import.vue"),
    },
    {
      path: "/settings",
      name: "Settings",
      component: () => import("@/views/Settings.vue"),
    },
    {
      path: "/projects/:id",
      name: "ProjectContext",
      component: () => import("@/views/projects/Layout.vue"),
      children: [
        {
          path: "",
          name: "ProjectContextHome",
          redirect: (to) => ({
            name: "ProjectOverview",
            params: { id: to.params.id },
          }),
        },
        {
          path: "overview",
          name: "ProjectOverview",
          component: () => import("@/views/projects/[id]/Overview.vue"),
        },
        {
          path: "history",
          name: "ProjectHistory",
          component: () => import("@/views/projects/[id]/History.vue"),
        },
        {
          path: "execute",
          name: "ProjectExecute",
          component: () => import("@/views/projects/[id]/Execute.vue"),
        },
        {
          path: "generate",
          name: "ProjectGenerate",
          component: () => import("@/views/projects/[id]/Generate.vue"),
        },
        {
          path: "coverage",
          name: "ProjectCoverage",
          component: () => import("@/views/projects/[id]/Coverage.vue"),
        },
      ],
    },
  ],
});

export default router;