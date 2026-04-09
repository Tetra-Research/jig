import { z } from "zod";

export const createProjectSchema = z.object({
  name: z.string().min(1),
});

export const updateProjectSchema = z.object({
  name: z.string().min(1).optional(),
  status: z.enum(["draft", "active", "archived"]).optional(),
});
