import type { Request, Response } from "express";
import { createProjectSchema } from "./schema";
import { updateProjectSchema } from "./schema";

export async function createProjectHandler(req: Request, res: Response) {
  const payload = createProjectSchema.parse(req.body);

  res.status(201).json({
    ok: true,
    payload,
  });
}

export async function updateProjectHandler(req: Request, res: Response) {
  const payload = updateProjectSchema.parse(req.body);

  res.json({
    ok: true,
    projectId: req.params.projectId,
    payload,
  });
}
