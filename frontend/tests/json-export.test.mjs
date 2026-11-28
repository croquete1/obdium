import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { saveJsonExport } from "../scripts/json-export.mjs";

test("manual export writes UTF-8 JSON to the selected path with Tauri 2 arguments", async () => {
  const writes = [];
  const data = [{ description: "Pressão — válida" }];
  const result = await saveJsonExport({
    data, defaultPath: "vin_details.json",
    save: async (options) => {
      assert.equal(options.defaultPath, "vin_details.json");
      return "/chosen/vin.json";
    },
    writeTextFile: async (...args) => { writes.push(args); },
  });
  assert.equal(result, true);
  assert.deepEqual(writes, [["/chosen/vin.json", JSON.stringify(data, null, 2)]]);
});

test("cancelled picker does not write", async () => {
  assert.equal(await saveJsonExport({
    data: [], defaultPath: "readiness_tests.json",
    save: async () => null,
    writeTextFile: async () => assert.fail("unexpected write"),
  }), false);
});

test("automatic DTC export uses fixed application data destination", async () => {
  const writes = [];
  await saveJsonExport({
    data: [], defaultPath: "dtc_log.json", automatic: true, appDataBaseDir: 14,
    save: async () => assert.fail("unexpected picker"),
    writeTextFile: async (...args) => { writes.push(args); },
  });
  assert.deepEqual(writes, [["dtc_log.json", "[]", { baseDir: 14 }]]);
});

test("automatic export cannot choose another destination or omit its base directory", async () => {
  for (const [defaultPath, appDataBaseDir] of [["other.json", 14], ["dtc_log.json", undefined]]) {
    await assert.rejects(saveJsonExport({
      data: [], defaultPath, appDataBaseDir, automatic: true,
      writeTextFile: async () => assert.fail("unexpected write"),
    }), /Unsupported automatic export destination/);
  }
});

test("write failure is propagated instead of reporting success", async () => {
  await assert.rejects(saveJsonExport({
    data: [], defaultPath: "dtc_log.json",
    save: async () => "/chosen/dtc.json",
    writeTextFile: async () => { throw new Error("permission denied"); },
  }), /permission denied/);
});

test("filesystem capability is limited to text writes and fixed automatic log scope", () => {
  const config = JSON.parse(readFileSync(new URL("../../backend/capabilities/migrated.json", import.meta.url), "utf8"));
  const fsPermissions = config.permissions.filter((p) => typeof p === "string" ? p.startsWith("fs:") : p.identifier.startsWith("fs:"));
  assert.deepEqual(fsPermissions, [
    "fs:allow-write-text-file",
    { identifier: "fs:scope", allow: [{ path: "$APPDATA/dtc_log.json" }] },
  ]);
  assert.deepEqual(config.windows, ["main"]);
  assert.equal(config.local, true);
});

test("all three production exports use the shared helper and no v1 write call", () => {
  const main = readFileSync(new URL("../scripts/main.js", import.meta.url), "utf8");
  const features = readFileSync(new URL("../scripts/features.js", import.meta.url), "utf8");
  assert.equal((main.match(/await saveJsonExport\(/g) || []).length, 2);
  assert.equal((features.match(/await saveJsonExport\(/g) || []).length, 1);
  assert.doesNotMatch(main + features, /\b(?:writeFile|removeFile)\s*\(/);
});

test("production DTC export refuses disconnected and empty snapshots", async () => {
  const source = readFileSync(new URL("../scripts/features.js", import.meta.url), "utf8");
  const start = source.indexOf("export async function exportDtcs(");
  const end = source.indexOf("export async function connectElm(", start);
  assert.ok(start >= 0 && end > start);
  const notices = [];
  const writes = [];
  const list = { childNodes: [] };
  const context = {
    dtcList: list,
    window: { connected: true, __TAURI__: { path: { BaseDirectory: { AppData: 14 } } } },
    addNotification: (...args) => notices.push(args),
    save: async () => "/chosen/dtc.json",
    writeTextFile: async (...args) => writes.push(args),
    saveJsonExport,
    console: { error: () => assert.fail("unexpected export failure") },
  };
  runInNewContext(source.slice(start, end).replace("export async", "async") +
    "\nthis.exportDtcs = exportDtcs;", context);
  await context.exportDtcs(false);
  await context.exportDtcs(true);
  assert.equal(writes.length, 0);
  assert.equal(notices.length, 1);
  list.childNodes.push({
    nodeType: 1,
    querySelector: (selector) => ({ textContent: selector === "#dtc-name" ? "P0123" : "fixture" }),
  });
  context.window.connected = false;
  await context.exportDtcs(false);
  await context.exportDtcs(true);
  assert.equal(writes.length, 0);
  assert.equal(notices.length, 2);
  context.window.connected = true;
  await context.exportDtcs(false);
  assert.equal(writes.length, 1);
  assert.equal(writes[0][0], "/chosen/dtc.json");
  assert.equal(JSON.parse(writes[0][1])[0].name, "P0123");
});

test("connection boundaries invalidate the DTC display", () => {
  const source = readFileSync(new URL("../scripts/events.js", import.meta.url), "utf8");
  for (const connected of [true, false]) {
    const anchor = "window.connected = " + connected + ";";
    const start = source.indexOf(anchor);
    assert.ok(start >= 0);
    const end = source.indexOf("DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)", start);
    assert.ok(end > start);
    const block = source.slice(start, source.indexOf(";", end) + 1);
    const list = { children: ["old code"], replaceChildren() { this.children = []; } };
    const header = { textContent: "old count" };
    runInNewContext(block, {
      window: {},
      replaceReadinessSnapshot() {},
      document: { getElementById: (id) => id === "dtc-list" ? list :
        id === "readiness-tests-list" ? { replaceChildren() {} } : header },
    });
    assert.equal(list.children.length, 0);
    assert.equal(header.textContent, "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)");
  }
});

test("pending DTC picker cannot export replaced or disconnected rows", async () => {
  const source = readFileSync(new URL("../scripts/features.js", import.meta.url), "utf8");
  const start = source.indexOf("export async function exportDtcs(");
  const end = source.indexOf("export async function connectElm(", start);
  assert.ok(start >= 0 && end > start);
  const row = () => ({
    nodeType: 1,
    querySelector: () => ({ textContent: "fixture" }),
  });
  const list = { childNodes: [row()] };
  const writes = [];
  let choosePath;
  const context = {
    dtcList: list,
    window: { connected: true, __TAURI__: { path: { BaseDirectory: { AppData: 14 } } } },
    addNotification() {},
    save: () => new Promise((resolve) => { choosePath = resolve; }),
    writeTextFile: async (...args) => writes.push(args),
    saveJsonExport,
    console: { error: () => assert.fail("unexpected export failure") },
  };
  runInNewContext(source.slice(start, end).replace("export async", "async") +
    "\nthis.exportDtcs = exportDtcs;", context);
  let pending = context.exportDtcs(false);
  list.childNodes = [row()];
  choosePath("/chosen/stale.json");
  await pending;
  assert.equal(writes.length, 0);
  pending = context.exportDtcs(false);
  context.window.connected = false;
  choosePath("/chosen/disconnected.json");
  await pending;
  assert.equal(writes.length, 0);
  context.window.connected = true;
  pending = context.exportDtcs(false);
  choosePath("/chosen/current.json");
  await pending;
  assert.equal(writes.length, 1);
  assert.equal(writes[0][0], "/chosen/current.json");
});
