import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const html = read("../index.html");
const main = read("../scripts/main.js");
const features = read("../scripts/features.js");
const eventSource = read("../scripts/events.js");
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
  const helperStart = features.indexOf("export async function requestDtcScan");
  const helperEnd = features.indexOf("export async function exportDtcs", helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart);
  const context = {
    window: { connected },
    document: { getElementById: (id) => elements[id] },
    emit: async (name) => {
      events.push(name);
      await emit(name, rows, header);
    },
    addNotification: (...args) => notices.push(args),
    console: { error: (...args) => errors.push(args) },
  };
  runInNewContext(
    features.slice(helperStart, helperEnd).replace("export async function", "async function") +
      "\n" + main.slice(start, end) + "\nthis.scan = requestDtcScan;",
    context,
  );
  assert.equal(typeof click, "function");
  return { click, scan: context.scan, window: context.window, notices, events, errors, rows, header };
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

test("automatic DTC scan uses the same request function with a session guard", async () => {
  const start = eventSource.indexOf("    if (window.autoCheckCodes) {");
  const end = eventSource.indexOf("    if (window.unitPreferences)", start);
  assert.ok(start >= 0 && end > start);
  const calls = [];
  const context = {
    window: { connected: true, autoCheckCodes: true },
    connectionGeneration: 3,
    generation: 3,
    requestDtcScan(options) { calls.push(options); },
  };
  runInNewContext(eventSource.slice(start, end), context);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].isCurrent(), true);
  context.connectionGeneration++;
  assert.equal(calls[0].isCurrent(), false);
  context.window.autoCheckCodes = false;
  runInNewContext(eventSource.slice(start, end), context);
  assert.equal(calls.length, 1);
});

test("shared DTC scan refuses an invalidated session before dispatch", async () => {
  const fixture = scanFixture();
  await fixture.scan({ isCurrent: () => false });
  assert.deepEqual(fixture.events, []);
  assert.deepEqual(fixture.notices, []);
  assert.deepEqual(fixture.rows, ["old diagnostic result"]);
});

test("late automatic DTC dispatch rejection does not notify a replacement session", async () => {
  let rejectDispatch;
  let current = true;
  const fixture = scanFixture({ emit: () => new Promise((_, reject) => { rejectDispatch = reject; }) });
  const pending = fixture.scan({ isCurrent: () => current });
  current = false;
  rejectDispatch(new Error("old session"));
  await pending;
  assert.deepEqual(fixture.notices, []);
  assert.equal(fixture.errors.length, 1);
});

test("late manual DTC dispatch rejection after disconnect does not notify", async () => {
  let rejectDispatch;
  const fixture = scanFixture({ emit: () => new Promise((_, reject) => { rejectDispatch = reject; }) });
  const pending = fixture.click();
  fixture.window.connected = false;
  rejectDispatch(new Error("disconnected"));
  await pending;
  assert.deepEqual(fixture.notices, []);
  assert.equal(fixture.errors.length, 1);
});
