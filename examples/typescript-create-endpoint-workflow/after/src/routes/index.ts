import { Router } from "express";
import { createProjectHandler } from "./projects/handler";

export const api = Router();
api.post("/projects", createProjectHandler);
