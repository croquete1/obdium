import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

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
