import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const apiUrl = "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits";

console.log("Codex Gauge reset credits diagnose");

try {
  const auth = readAuthJson();
  const tokens = auth.tokens && typeof auth.tokens === "object" ? auth.tokens : {};
  const accessToken = typeof tokens.access_token === "string" ? tokens.access_token : null;

  if (!accessToken) {
    printStep(false, "未检测到 Codex 登录状态");
    process.exit(1);
  }

  const headers = {
    Authorization: `Bearer ${accessToken}`,
    "OpenAI-Beta": "codex-1",
    originator: "Codex Desktop",
  };
  if (typeof tokens.account_id === "string") {
    headers["ChatGPT-Account-ID"] = tokens.account_id;
  }

  const response = await fetch(apiUrl, { headers });
  if (response.status === 401) {
    printStep(false, "401: 凭证失效或未正确携带 Authorization header");
    process.exit(1);
  }
  if (!response.ok) {
    printStep(false, `request_failed: status ${response.status}`);
    process.exit(1);
  }

  const payload = await response.json();
  const credits = Array.isArray(payload.credits) ? payload.credits : [];

  printStep(true, `available_count=${numberOrUnknown(payload.available_count)}`);
  if (!credits.length) {
    console.log("credits: none");
  }

  credits.forEach((credit, index) => {
    console.log(
      [
        `credit #${index + 1}`,
        `status=${safeText(credit.status)}`,
        `title=${safeText(credit.title)}`,
        `granted_at=${formatLocalTime(credit.granted_at)}`,
        `expires_at=${formatLocalTime(credit.expires_at)}`,
      ].join(" | "),
    );
  });
} catch {
  printStep(false, "Codex reset credits 查询失败");
  process.exit(1);
}

function readAuthJson() {
  const authPath = process.env.CODEX_HOME
    ? path.join(process.env.CODEX_HOME, "auth.json")
    : path.join(os.homedir(), ".codex", "auth.json");
  return JSON.parse(fs.readFileSync(authPath, "utf8"));
}

function printStep(ok, message) {
  console.log(`${ok ? "[OK]" : "[FAIL]"} ${message}`);
}

function numberOrUnknown(value) {
  return typeof value === "number" ? value : "未知";
}

function safeText(value) {
  return typeof value === "string" && value.trim() ? value.trim() : "未知";
}

function formatLocalTime(value) {
  if (!value) return "未知";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "未知";
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(date);
}
