import { z } from "zod";

export const projectSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1),
  status: z.enum(["draft", "active", "archived"]),
});

export type Project = z.infer<typeof projectSchema>;
