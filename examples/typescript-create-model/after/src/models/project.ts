import { z } from "zod";

export const projectSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1),
  createdAt: z.string().datetime(),
});

export type Project = z.infer<typeof projectSchema>;
