import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
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
