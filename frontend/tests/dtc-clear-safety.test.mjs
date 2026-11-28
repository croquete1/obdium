import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const html = read("../index.html");
const main = read("../scripts/main.js");
const features = read("../scripts/features.js");
const bridge = read("../../backend/src/bridge/events.rs");

test("DTC clear is a native disabled control with a visible explanation", () => {
  const button = html.match(/<button\b[^>]*\bid="dtc-clear-button"[^>]*>/s);
  assert.ok(button, "expected native DTC clear button");
  assert.match(button[0], /\bdisabled\b/);
  assert.match(button[0], /aria-describedby="dtc-clear-status"/);
  assert.match(html, /id="dtc-clear-status"[\s\S]*?temporarily unavailable/);
  assert.match(html, /id="dtc-scan-button"/);
  assert.match(html, /id="dtc-log-file"/);
});

test("legacy optimistic clear flow cannot dispatch or mutate the list", () => {
  assert.doesNotMatch(main, /\bclearDtcs\b|\bdtcClearButton\b/);
  assert.doesNotMatch(features, /\bclearDtcs\b|["']clear-dtcs["']/);
  assert.doesNotMatch(features, /Cleared all pending diagnostic trouble codes/);
});

test("legacy backend clear event is not registered or dispatched", () => {
  assert.doesNotMatch(bridge, /\blisten_clear_dtcs\b|["']clear-dtcs["']/);
  assert.doesNotMatch(bridge, /\.clear_trouble_codes\s*\(/);
  assert.match(bridge, /window\.listen\("get-dtcs"/);
});

function scanFixture({ connected = true, emit = async () => {} } = {}) {
  const start = main.indexOf("const dtcScanButton =");
  const end = main.indexOf("const dtcLogButton =", start);
  assert.ok(start >= 0 && end > start);
  let click;
  const notices = [];
  const events = [];
  const errors = [];
  const rows = ["old diagnostic result"];
  const header = { textContent: "DIAGNOSTIC TROUBLE CODES (1)" };
  const elements = {
    "dtc-scan-button": { addEventListener(name, handler) {
      assert.equal(name, "click");
      click = handler;
    } },
    "dtc-list": { replaceChildren() { rows.length = 0; } },
    "dtc-header": header,
  };
  runInNewContext(main.slice(start, end), {
    window: { connected },
    document: { getElementById: (id) => elements[id] },
    emit: async (name) => {
      events.push(name);
      await emit(name, rows, header);
    },
    addNotification: (...args) => notices.push(args),
    console: { error: (...args) => errors.push(args) },
  });
  assert.equal(typeof click, "function");
  return { click, notices, events, errors, rows, header };
}

test("manual DTC scan refuses disconnected requests without claiming success", async () => {
  const fixture = scanFixture({ connected: false });
  await fixture.click();
  assert.deepEqual(fixture.events, []);
  assert.equal(fixture.notices[0][0], "SCAN UNAVAILABLE");
  assert.equal(fixture.notices.length, 1);
});

test("manual DTC scan invalidates old results before dispatch and does not claim completion", async () => {
  const fixture = scanFixture({
    emit: async (name, rows, header) => {
      assert.equal(name, "get-dtcs");
      assert.equal(rows.length, 0);
      assert.equal(header.textContent, "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)");
    },
  });
  await fixture.click();
  assert.deepEqual(fixture.events, ["get-dtcs"]);
  assert.deepEqual(fixture.notices, []);
  assert.equal(fixture.rows.length, 0);
  assert.equal(fixture.header.textContent, "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)");
});

test("manual DTC scan dispatch rejection is reported without restoring stale results", async () => {
  const fixture = scanFixture({ emit: async () => { throw new Error("IPC rejected"); } });
  await fixture.click();
  assert.deepEqual(fixture.events, ["get-dtcs"]);
  assert.equal(fixture.notices.length, 1);
  assert.equal(fixture.notices[0][0], "SCAN REQUEST FAILED");
  assert.equal(fixture.rows.length, 0);
  assert.equal(fixture.errors.length, 1);
  assert.equal(fixture.header.textContent, "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)");
});

test("manual DTC scan does not overwrite results arriving before IPC acceptance", async () => {
  const fixture = scanFixture({
    emit: async (name, rows, header) => {
      rows.push("new diagnostic result");
      header.textContent = "DIAGNOSTIC TROUBLE CODES (1)";
    },
  });
  await fixture.click();
  assert.deepEqual(fixture.rows, ["new diagnostic result"]);
  assert.equal(fixture.header.textContent, "DIAGNOSTIC TROUBLE CODES (1)");
  assert.deepEqual(fixture.notices, []);
});
