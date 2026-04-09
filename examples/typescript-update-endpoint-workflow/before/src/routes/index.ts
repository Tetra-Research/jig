import { Router } from "express";
import { createProjectHandler } from "./projects/handler";

export const routes = [
  router.post("/projects", createProjectHandler),
];
