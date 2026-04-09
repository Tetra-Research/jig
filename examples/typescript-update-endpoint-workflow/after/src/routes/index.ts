import { Router } from "express";
import { createProjectHandler } from "./projects/handler";
import { updateProjectHandler } from "./projects/handler";

export const routes = [
  router.post("/projects", createProjectHandler),
  router.put("/projects/:projectId", updateProjectHandler),
];
