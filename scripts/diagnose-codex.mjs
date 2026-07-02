import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const requestedCommand = process.argv[2] || process.env.CODEX_COMMAND || "codex";
const command = resolveCodexCommand(requestedCommand);
const timeoutMs = 8_000;

console.log("Codex Gauge diagnose");
console.log(`command: ${requestedCommand}`);
if (command !== requestedCommand) console.log(`resolved: ${command}`);

const version = await runCommand(command, ["--version"], 5_000);
printStep("codex --version", version.ok, version.message);

const server = spawnSafe(command, ["app-server"], {
  stdio: ["pipe", "pipe", "ignore"],
  windowsHide: true,
});

if (!server.ok) {
  printStep("codex app-server", false, safeError(server.error));
  process.exitCode = 1;
  process.exit();
}

const appServer = server.child;

let nextId = 1;
const pending = new Map();
let buffer = "";

appServer.on("error", (error) => {
  for (const item of pending.values()) item.reject(error);
  pending.clear();
});

appServer.stdout.on("data", (chunk) => {
  buffer += chunk.toString("utf8");
  let index = buffer.indexOf("\n");
  while (index >= 0) {
    const line = buffer.slice(0, index).trim();
    buffer = buffer.slice(index + 1);
    index = buffer.indexOf("\n");
    if (!line) continue;

    try {
      const message = JSON.parse(line);
      const item = pending.get(message.id);
      if (!item) continue;
      pending.delete(message.id);
      if (message.error) item.reject(new Error("method_error"));
      else item.resolve(message.result ?? message);
    } catch {
      // 诊断脚本不输出原始响应，避免泄露账号信息。
    }
  }
});

try {
  const initialized = await initialize(appServer);
  printStep("initialize / initialized", initialized.ok, initialized.message);

  for (const method of ["account/read", "account/rateLimits/read", "account/usage/read"]) {
    const result = await request(appServer, method).then(
      (value) => ({ ok: true, value }),
      (error) => ({ ok: false, error }),
    );
    printMethod(method, result);
  }
} catch (error) {
  printStep("codex app-server", false, safeError(error));
} finally {
  appServer.kill();
}

function initialize(child) {
  return request(child, "initialize", {
    clientInfo: { name: "codex-gauge-diagnose", version: "0.1.0" },
    capabilities: {},
    protocolVersion: "2024-11-05",
  })
    .then(() => {
      send(child, {
        jsonrpc: "2.0",
        method: "initialized",
        params: {},
      });
      return { ok: true, message: "ok" };
    })
    .catch((error) => ({ ok: false, message: safeError(error) }));
}

function request(child, method, params) {
  const id = nextId++;
  const payload = {
    jsonrpc: "2.0",
    id,
    method,
    ...(params ? { params } : {}),
  };

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error("timeout"));
    }, timeoutMs);

    pending.set(id, {
      resolve: (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      reject: (error) => {
        clearTimeout(timer);
        reject(error);
      },
    });

    send(child, payload);
  });
}

function send(child, payload) {
  child.stdin.write(`${JSON.stringify(payload)}\n`);
}

function printMethod(method, result) {
  if (!result.ok) {
    printStep(method, false, safeError(result.error));
    return;
  }

  const value = result.value;
  const keys = value && typeof value === "object" ? Object.keys(value).slice(0, 8).join(", ") : "";
  const email = findEmail(value);
  const suffix = email ? `, email=${maskEmail(email)}` : "";
  printStep(method, true, keys ? `keys: ${keys}${suffix}` : `ok${suffix}`);
}

function runCommand(command, args, timeout) {
  return new Promise((resolve) => {
    const spawned = spawnSafe(command, args, { windowsHide: true });
    if (!spawned.ok) {
      resolve({ ok: false, message: safeError(spawned.error) });
      return;
    }

    const child = spawned.child;
    let done = false;
    let stdout = "";
    let stderr = "";

    const timer = setTimeout(() => finish(false, "timeout"), timeout);
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", (error) => finish(false, safeError(error)));
    child.on("close", (code) =>
      finish(code === 0, code === 0 ? firstLine(stdout) : firstLine(stderr) || `exit ${code}`),
    );

    function finish(ok, message) {
      if (done) return;
      done = true;
      clearTimeout(timer);
      child.kill();
      resolve({ ok, message });
    }
  });
}

function spawnSafe(command, args, options) {
  try {
    return { ok: true, child: spawn(command, args, options) };
  } catch (error) {
    return { ok: false, error };
  }
}

function resolveCodexCommand(command) {
  if (command !== "codex" && command !== "codex.exe") return command;

  for (const candidate of bundledCodexCandidates()) {
    if (fs.existsSync(candidate)) return candidate;
  }

  return command;
}

function bundledCodexCandidates() {
  const localAppData = process.env.LOCALAPPDATA;
  if (!localAppData) return [];

  return [
    path.join(localAppData, "OpenAI", "Codex", "bin", "codex.exe"),
    path.join(
      localAppData,
      "Packages",
      "OpenAI.Codex_2p2nqsd0c76g0",
      "LocalCache",
      "Local",
      "OpenAI",
      "Codex",
      "bin",
      "codex.exe",
    ),
  ];
}

function printStep(name, ok, message) {
  console.log(`${ok ? "[OK]" : "[FAIL]"} ${name}: ${message || ""}`);
}

function firstLine(value) {
  return value.trim().split(/\r?\n/)[0] || "";
}

function safeError(error) {
  if (!error) return "unknown";
  if (error.code === "ENOENT") return "codex_not_found";
  if (error.code === "EACCES" || error.code === "EPERM") return "permission_denied";
  if (error.message === "timeout") return "timeout";
  if (error.message === "method_error") return "method_error";
  return "app_server_error";
}

function findEmail(value) {
  if (!value || typeof value !== "object") return null;
  for (const [key, item] of Object.entries(value)) {
    if (/email/i.test(key) && typeof item === "string") return item;
    if (item && typeof item === "object") {
      const found = findEmail(item);
      if (found) return found;
    }
  }
  return null;
}

function maskEmail(email) {
  const [name, domain] = email.split("@");
  if (!domain) return "***";
  return `${name.slice(0, 1)}***@${domain}`;
}
