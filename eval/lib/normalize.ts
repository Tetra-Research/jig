export function normalizeFile(content: string): string {
  return content
    .replace(/\r\n/g, "\n")          // normalize line endings
    .split("\n")
    .map((line) => line.trimEnd())   // strip trailing whitespace per line
    .join("\n")
    .replace(/\n+$/, "\n");          // single trailing newline
}
