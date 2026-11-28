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

test("VIN result ignores a different query and rejects empty or malformed replies", () => {
  const container = node();
  const button = { disabled: true };
  const input = { value: "SYNTHETIC-FIXTURE" };
  const handler = eventHandler("decode-vin-result", 'listen("display-notification"', {
    exportButton: button,
    document: { getElementById: () => container, querySelector: () => input, createElement: node },
    addNotification() {},
  });
  handler({ payload: { vin: input.value, make: "Fixture", model: "N/A", unused: 4, error_msg: "Decoded successfully." } });
  assert.equal(button.disabled, false);
  assert.equal(container.children.length, 2);
  handler({ payload: { vin: "OTHER-FIXTURE", make: "Other" } });
  assert.equal(container.children.length, 2);
  assert.equal(button.disabled, false);
  handler({ payload: { vin: "", make: "", error_msg: "Unavailable" } });
  assert.equal(container.children.length, 0);
  assert.equal(button.disabled, true);
  handler(undefined);
  assert.equal(button.disabled, true);
});
