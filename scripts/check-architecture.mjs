import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("../src", import.meta.url));
const rustRoot = fileURLToPath(new URL("../src-tauri/src", import.meta.url));
const frontendInvokeEntry = "src/lib/invoke.ts";
const rustCommandEntry = "src-tauri/src/commands.rs";

function toProjectPath(file) {
  return relative(process.cwd(), file).split(sep).join("/");
}

function collectFiles(root, extension) {
  const stack = [root];
  const files = [];

  while (stack.length > 0) {
    const current = stack.pop();

    for (const entry of readdirSync(current)) {
      const next = join(current, entry);
      const stats = statSync(next);

      if (stats.isDirectory()) {
        stack.push(next);
        continue;
      }

      if (next.endsWith(extension)) {
        files.push(next);
      }
    }
  }

  return files;
}

const frontendViolations = collectFiles(frontendRoot, ".ts")
  .concat(collectFiles(frontendRoot, ".tsx"))
  .filter((file) => toProjectPath(file) !== frontendInvokeEntry)
  .filter((file) => {
    const content = readFileSync(file, "utf8");
    return (
      content.includes("@tauri-apps/api/core") || content.includes("invoke(")
    );
  })
  .map((file) => toProjectPath(file));

const rustViolations = collectFiles(rustRoot, ".rs")
  .filter((file) => toProjectPath(file) !== rustCommandEntry)
  .filter((file) => readFileSync(file, "utf8").includes("#[tauri::command]"))
  .map((file) => toProjectPath(file));

if (frontendViolations.length > 0 || rustViolations.length > 0) {
  const lines = [];

  if (frontendViolations.length > 0) {
    lines.push(
      `Direct invoke usage is only allowed in ${frontendInvokeEntry}: ${frontendViolations.join(
        ", ",
      )}`,
    );
  }

  if (rustViolations.length > 0) {
    lines.push(
      `#[tauri::command] is only allowed in ${rustCommandEntry}: ${rustViolations.join(", ")}`,
    );
  }

  console.error(lines.join("\n"));
  process.exit(1);
}
