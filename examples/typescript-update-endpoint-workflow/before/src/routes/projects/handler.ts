import type { Request, Response } from "express";
import { createProjectSchema } from "./schema";

export async function createProjectHandler(req: Request, res: Response) {
  const payload = createProjectSchema.parse(req.body);

  res.status(201).json({
    ok: true,
    payload,
  });
}
