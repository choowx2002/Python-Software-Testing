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
      path: "/debug/schema",
      name: "DatabaseSchema",
      component: () => import("@/components/DatabaseSchemaViewer.vue"),
    },
    {
      path: "/projects/:id",
      name: "ProjectContext",
      component: () => import("@/views/projects/Layout.vue"),
      children: [
        {
          path: "",
          redirect: (to) => ({
            name: "ProjectExecute",
            params: { id: to.params.id },
          }),
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