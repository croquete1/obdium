import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { replaceReadinessSnapshot, readinessLabels } from "../scripts/readiness-state.mjs";

const eventsSource = readFileSync(new URL("../scripts/events.js", import.meta.url), "utf8");
const featuresSource = readFileSync(new URL("../scripts/features.js", import.meta.url), "utf8");

// Use harmless markup-like text to verify literal rendering without executable content.
const literal = "<b>sample & details</b>";

function node() {
  const children = [];
  const fields = new Map();
  const values = Array.from({ length: 5 }, () => ({ textContent: "" }));
  return {
    children,
    style: {},
    textContent: "",
    innerHTML: "",
    appendChild(child) { children.push(child); },
    replaceChildren() { children.length = 0; },
    querySelector(selector) {
      if (!fields.has(selector)) fields.set(selector, { textContent: "", style: {}, addEventListener() {} });
      return fields.get(selector);
    },
    querySelectorAll(selector) {
      assert.equal(selector, ".pid-value");
      return values;
    },
  };
}

function eventHandler(name, nextAnchor, context) {
  const start = eventsSource.indexOf('listen("' + name + '",');
  const end = eventsSource.indexOf(nextAnchor, start);
  assert.ok(start >= 0 && end > start);
  let handler;
  runInNewContext(eventsSource.slice(start, end), {
    ...context,
    listen(event, callback) { assert.equal(event, name); handler = callback; },
  });
  assert.equal(typeof handler, "function");
  return handler;
}

test("PID fields are literal text while static markup and supported opacity remain", () => {
  const list = node();
  const header = node();
  const handler = eventHandler("update-pids", "const dtcList", {
    document: { createElement: node, getElementById: (id) => id === "pid-list" ? list : header },
    listenExpandPID() {},
    addGraphDropdownOption() {},
    addNotification() {},
  });
  handler({ payload: [{ supported: false, pidName: literal, mode: literal, pid: literal, formula: literal, unit: literal }] });
  const row = list.children[0];
  assert.ok(!row.innerHTML.includes(literal));
  assert.equal(row.querySelector(".info-row .name").textContent, "[UNSUPPORTED]  " + literal.toUpperCase());
  assert.equal(row.querySelector(".pid-container").style.opacity, "0.15");
  assert.deepEqual(row.querySelectorAll(".pid-value").map((field) => field.textContent),
    ["$" + literal, literal, literal + literal, literal, literal.toUpperCase()]);
});

test("DTC fields are rendered literally", () => {
  const list = node();
  const handler = eventHandler("update-dtcs", "const serialPortMenu", {
    dtcList: list, dtcHeader: node(),
    document: { createElement: node },
    window: { connected: true, autoSaveCodes: false },
  });
  handler({ payload: [{ category: literal, name: literal, location: literal, description: literal, permanant: true }] });
  const row = list.children[0];
  assert.ok(!row.innerHTML.includes(literal));
  for (const field of ["category", "name", "location"]) {
    assert.equal(row.querySelector("#dtc-" + field).textContent, literal);
  }
  assert.equal(row.querySelector("#dtc-description").textContent, literal + " [PERMANENT CODE]");
});

test("readiness name is rendered literally", () => {
  const list = node();
  const handler = eventHandler("update-readiness-tests", 'listen("update-command-output"', {
    readinessTests: list, document: { createElement: node }, addNotification() {},
    window: { connected: true }, replaceReadinessSnapshot, readinessLabels,
  });
  handler({ payload: [{ name: literal, available: true, complete: false }] });
  const row = list.children[0];
  assert.ok(!row.innerHTML.includes(literal));
  assert.equal(row.querySelector("#test-name").textContent, "TEST: " + literal);
  assert.equal(row.querySelector("#test-availability").textContent, "SUPPORTED");
  assert.equal(row.querySelector("#test-completeness").textContent, "NOT READY");
});

test("notification title and body are literal text", () => {
  const list = node();
  const start = featuresSource.indexOf("export function addNotification(");
  const end = featuresSource.indexOf("export function listenExpandPID", start);
  assert.ok(start >= 0 && end > start);
  const code = featuresSource.slice(start, end).replace("export function", "function");
  runInNewContext(code + "\naddNotification(literal, literal);", {
    literal, document: { createElement: node, getElementById: () => list },
    window: { hideNotifications: false }, console: { log() {} }, setTimeout() {},
  });
  const row = list.children[0];
  assert.ok(!row.innerHTML.includes(literal));
  assert.equal(row.querySelector(".notification-title").textContent, literal);
  assert.equal(row.querySelector(".notification-desc").textContent, literal);
});

test("HTML templates in the two production modules contain no interpolation", () => {
  for (const source of [eventsSource, featuresSource]) {
    const templates = [...source.matchAll(/\.innerHTML\s*=\s*`([\s\S]*?)`/g)];
    assert.ok(templates.length > 0);
    for (const match of templates) assert.ok(!match[1].includes("${"));
  }
});

test("malformed DTC snapshots are unavailable and never exported", () => {
  const list = node();
  const header = node();
  let exports = 0;
  const connection = { connected: true, autoSaveCodes: true };
  const handler = eventHandler("update-dtcs", "const serialPortMenu", {
    dtcList: list, dtcHeader: header,
    document: { createElement: node },
    window: connection,
    exportDtcs() { exports++; },
  });
  const valid = { category: "P", name: "P0123", location: "Powertrain",
    description: "Synthetic fixture", permanant: false };
  for (const event of [undefined, {}, { payload: null }, { payload: {} },
    { payload: "NO DATA" }, { payload: [valid, null] },
    { payload: [{ ...valid, permanant: "false" }] },
    { payload: [{ ...valid, name: "" }] }]) {
    list.appendChild(node());
    handler(event);
    assert.equal(header.textContent, "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)");
    assert.equal(list.children.length, 0);
    assert.equal(exports, 0);
  }
  handler({ payload: [valid] });
  assert.equal(header.textContent, "DIAGNOSTIC TROUBLE CODES (1)");
  assert.equal(list.children[0].querySelector("#dtc-description").textContent,
    "Synthetic fixture [NON-PERMANENT CODE]");
  assert.equal(exports, 1);
  handler({ payload: [] });
  assert.equal(header.textContent, "DIAGNOSTIC TROUBLE CODES (NONE RETURNED — UNVERIFIED)");
  assert.equal(list.children.length, 0);
  assert.equal(exports, 1);
  connection.connected = false;
  handler({ payload: [valid] });
  assert.equal(list.children.length, 0);
  assert.equal(exports, 1);
});

test("VIN result accepts only the pending request, including same-VIN retries and errors", () => {
  const container = node();
  const button = { disabled: true };
  const input = { value: "SYNTHETIC-FIXTURE" };
  const ticket = (id) => ({ id, started: 100, deadline: 10100, timer: 1 });
  const state = { pendingVinRequest: ticket("new") };
  const status = node();
  let now = 100;
  const handler = eventHandler("decode-vin-result", 'listen("display-notification"', {
    exportButton: button, window: state,
    document: { getElementById: (id) => id === "vin-container" ? container : status,
      querySelector: () => input, createElement: node },
    clearTimeout() {}, performance: { now: () => now },
    addNotification() {},
  });
  handler({ payload: { request_id: "old", vin: input.value, make: "Old" } });
  handler({ payload: { request_id: "old", vin: "", error_msg: "Old error" } });
  assert.equal(state.pendingVinRequest.id, "new");
  assert.equal(button.disabled, true);
  handler({ payload: { request_id: "new", vin: input.value, make: "Fixture", model: "N/A", unused: 4, error_msg: "Decoded successfully." } });
  assert.equal(button.disabled, false);
  assert.equal(container.children.length, 2);
  assert.equal(state.pendingVinRequest, null);
  handler({ payload: { request_id: "new", vin: "", error_msg: "Duplicate" } });
  handler(undefined);
  assert.equal(container.children.length, 2);
  assert.equal(button.disabled, false);
  state.pendingVinRequest = ticket("next");
  handler({ payload: { request_id: "next", vin: "", error_msg: "Unavailable" } });
  assert.equal(container.children.length, 0);
  assert.equal(button.disabled, true);
  state.pendingVinRequest = ticket("mismatch");
  handler({ payload: { request_id: "mismatch", vin: "OTHER-FIXTURE", make: "Other" } });
  assert.equal(container.children.length, 0);
  assert.equal(button.disabled, true);
  for (const invalidTime of [99, 10100, 10101, NaN]) {
    state.pendingVinRequest = ticket("expired");
    now = invalidTime;
    handler({ payload: { request_id: "expired", vin: input.value } });
    assert.equal(container.children.length, 0);
    assert.equal(button.disabled, true);
    assert.equal(state.pendingVinRequest, null);
    assert.match(status.textContent, /timed out/);
  }
  now = 10099;
  state.pendingVinRequest = ticket("last-valid");
  handler({ payload: { request_id: "last-valid", vin: input.value } });
  assert.equal(button.disabled, false);
});

test("VIN input and new queries revoke outstanding request identity", async () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const start = html.indexOf("const VIN_RESPONSE_TIMEOUT_MS");
  const end = html.indexOf("/// Toggle different screens", start);
  assert.ok(start >= 0 && end > start);
  const state = {};
  const button = { disabled: false };
  const input = { value: "synthetic-fixture", addEventListener(name, callback) { this.input = callback; } };
  const decode = { addEventListener(name, callback) { this.click = callback; } };
  const container = node();
  const requests = [];
  const timers = [];
  const cleared = [];
  const status = node();
  let seed = 0;
  runInNewContext(html.slice(start, end), {
    window: state, vinExportBtn: button, vinInput: input, decodeVinBtn: decode,
    document: { getElementById: (id) => id === "vin-container" ? container : status },
    crypto: { getRandomValues(bytes) { bytes.fill(++seed); return bytes; } },
    performance: { now: () => 100 },
    setTimeout(callback, delay) { assert.equal(delay, 10000); timers.push(callback); return timers.length; },
    clearTimeout(id) { cleared.push(id); },
    emit: async (name, payload) => requests.push({ name, payload }),
  });
  await decode.click();
  const first = state.pendingVinRequest;
  await decode.click();
  assert.notEqual(state.pendingVinRequest, first);
  assert.equal(requests.length, 2);
  assert.equal(requests[1].payload.vin, "SYNTHETIC-FIXTURE");
  assert.equal(requests[1].payload.request_id, state.pendingVinRequest.id);
  assert.equal(state.pendingVinRequest.deadline, 10100);
  timers[0]();
  assert.equal(state.pendingVinRequest.id, requests[1].payload.request_id);
  timers[1]();
  assert.equal(state.pendingVinRequest, null);
  assert.match(status.textContent, /timed out/);
  await decode.click();
  input.input();
  assert.equal(state.pendingVinRequest, null);
  timers[2]();
  assert.equal(status.textContent, "Enter a VIN to begin.");
  assert.equal(button.disabled, true);
  assert.deepEqual(cleared, [1, 2, 3]);
});
