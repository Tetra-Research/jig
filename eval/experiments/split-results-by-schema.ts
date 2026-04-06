import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { readResults, ResultSchemaError, formatDiagnosticsSummary } from "../harness/results.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EVAL_ROOT = path.resolve(__dirname, "..");

const { values: args } = parseArgs({
  options: {
    input: { type: "string", default: path.join(EVAL_ROOT, "results", "results.jsonl") },
    "out-dir": { type: "string" },
    prefix: { type: "string" },
  },
  strict: true,
});

const inputPath = args.input!;
const outDir = args["out-dir"] ?? path.dirname(inputPath);
const prefix = args.prefix ?? path.basename(inputPath, ".jsonl");

let loaded;
try {
  loaded = readResults(inputPath, { schemaMode: "compat" });
} catch (err) {
  if (err instanceof ResultSchemaError) {
    console.error(err.message);
    for (const line of formatDiagnosticsSummary(err.diagnostics)) {
      console.error(line);
    }
    process.exit(1);
  }
  throw err;
}

for (const line of formatDiagnosticsSummary(loaded.diagnostics)) {
  console.error(line);
}

if (loaded.diagnostics.malformed_lines > 0 || loaded.diagnostics.invalid_rows > 0) {
  console.error("[eval] Refusing to split: file contains malformed/invalid rows that need manual cleanup first.");
  process.exit(1);
}

if (loaded.entries.length === 0) {
  console.error("[eval] No valid rows found to split.");
  process.exit(1);
}

const v1Rows = loaded.entries.filter((entry) => entry.schema === "v1_legacy").map((entry) => entry.result);
const v2Rows = loaded.entries.filter((entry) => entry.schema === "v2_current").map((entry) => entry.result);

fs.mkdirSync(outDir, { recursive: true });

if (v1Rows.length > 0) {
  const outputPath = path.join(outDir, `${prefix}.v1-legacy.jsonl`);
  fs.writeFileSync(outputPath, v1Rows.map((row) => JSON.stringify(row)).join("\n") + "\n");
  console.log(`Wrote ${v1Rows.length} rows to ${outputPath}`);
}

if (v2Rows.length > 0) {
  const outputPath = path.join(outDir, `${prefix}.v2-current.jsonl`);
  fs.writeFileSync(outputPath, v2Rows.map((row) => JSON.stringify(row)).join("\n") + "\n");
  console.log(`Wrote ${v2Rows.length} rows to ${outputPath}`);
}

if (v1Rows.length === 0 || v2Rows.length === 0) {
  console.error("[eval] Input was already schema-homogeneous; only one output file was written.");
}
