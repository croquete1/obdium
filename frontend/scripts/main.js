const { listen, emit } = window.__TAURI__.event;
const { save } = window.__TAURI__.dialog;
const { writeTextFile } = window.__TAURI__.fs;
import { saveJsonExport } from "./json-export.mjs";
import { getReadinessSnapshot, getReadinessSnapshotGuard } from "./readiness-state.mjs";

import {
  exportDtcs,
  connectElm,
  disconnectElm,
  clearObdView,
  appendTerminalOutput,
  addNotification,
  addCustomPIDRow,
} from "./features.js";

import { saveUnitPreference } from "./settings.js";

// ELM connection
// Changes when connection-status is fired
let connected = false;
window.connectionConfig = {
  serialPort: "0",
  baudRate: "0",
  protocol: 0,
};

// Personal settings
let hideVin = false;
let deleteLogsOnExit = false;
let autoCheckCodes = false;
let autoSaveCodes = false;
let logFilePath = "";

// UI Components
const dropdowns = document.querySelectorAll(".dropdown");
const graphDropdowns = document.querySelectorAll(".graph-dropdown");
const connectButton = document.getElementById("btn-connect");
const disconnectButton = document.getElementById("btn-disconnect");
const clearObdButton = document.getElementById("obd-clear");
const pauseObdButton = document.getElementById("obd-pause");

// When frontend gets loaded
// alert the backend with an event.
window.addEventListener("DOMContentLoaded", () => {
  emit("frontend-loaded");

  // load serial ports
  emit("get-serial-ports");

  emit("get-connection-status");
});

function handleDropdown(dropdown, toggleName, menuName) {
  const toggle = dropdown.querySelector(toggleName);
  const menu = dropdown.querySelector(menuName);

  toggle.addEventListener("click", (e) => {
    e.stopPropagation();
    if (menu.style.display === "block") {
      menu.style.display = "none";
    } else {
      document.querySelectorAll(menuName).forEach((m) => {
        m.style.display = "none";
      });
      menu.style.display = "block";
    }
  });

  menu.addEventListener("click", (e) => {
    if (e.target.tagName === "LI") {
      toggle.textContent = e.target.textContent;
      toggle.dataset.value = e.target.dataset.value;
      menu.style.display = "none";

      if (toggle.id == "unit-preference" && window.unitPreferences) {
        // change unit
        const unitType = toggle.getAttribute("data-target");
        const unit = toggle.dataset.value;

        saveUnitPreference(unitType, unit);
      }
    }
  });
}

clearObdButton.addEventListener("click", clearObdView);
pauseObdButton.addEventListener("click", () => {
  window.obdViewPaused = !window.obdViewPaused;
  if (obdViewPaused) {
    pauseObdButton.textContent = "RESUME";
  } else {
    pauseObdButton.textContent = "PAUSE";
  }
});

dropdowns.forEach((dropdown) => {
  if (dropdown.id == "baud-rate-dropdown") {
    return;
  }

  handleDropdown(dropdown, ".dropdown-toggle", ".dropdown-menu");
});

graphDropdowns.forEach((dropdown) => {
  handleDropdown(dropdown, ".graph-dropdown-toggle", ".graph-dropdown-menu");
});

document.addEventListener("click", () => {
  document.querySelectorAll(".dropdown-menu").forEach((menu) => {
    menu.style.display = "none";
  });
});

connectButton.addEventListener("click", async () => {
  const baudRate = document.getElementById("baud-rate-selected");
  const serialPort = document.getElementById("serial-port-selected");
  const protocol = document.getElementById("protocol-selected");

  connectElm(
    baudRate.textContent.trim(),
    serialPort.textContent.trim(),
    parseInt(protocol.dataset.value),
  );
});

disconnectButton.addEventListener("click", disconnectElm);

const dtcScanButton = document.getElementById("dtc-scan-button");
dtcScanButton.addEventListener("click", async () => {
  await new Promise((r) => setTimeout(r, 500));

  addNotification("TROUBLE CODES", "Scanned for diagnostic trouble codes.");

  emit("get-dtcs");
});

const dtcLogButton = document.getElementById("dtc-log-file");
const dtcList = document.getElementById("dtc-list");
dtcLogButton.addEventListener("click", () => exportDtcs(false));

const logFileButton = document.getElementById("log-file-button");
logFileButton.addEventListener("click", async () => {
  window.logFilePath = await save({
    title: "Save as JSON",
    defaultPath: "requests.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });

  // set default path
  if (!window.logFilePath) {
    window.logFilePath = "./requests.json";
    return;
  }

  document.getElementById("log-file-path").textContent = window.logFilePath;
});

const imTestRefreshButton = document.getElementById("readiness-test-refresh");
const imTestExportButton = document.getElementById("readiness-test-export");

imTestRefreshButton.addEventListener("click", () => {
  emit("get-readiness-tests");
});

imTestExportButton.addEventListener("click", async () => {
  const isSnapshotCurrent = getReadinessSnapshotGuard();
  const totalJSON = getReadinessSnapshot();
  if (!window.connected || totalJSON.length === 0) {
    addNotification("EXPORT UNAVAILABLE", "Readiness information is not available for the current connection.");
    return;
  }

  try {
    await saveJsonExport({
      data: totalJSON,
      defaultPath: "readiness_tests.json",
      save,
      writeTextFile,
      isCurrent: () => window.connected && isSnapshotCurrent(),
    });
  } catch (error) {
    addNotification("EXPORT FAILED", "Could not save readiness tests.");
    console.error("JSON export failed", error);
  }
});

const vinExportButton = document.getElementById("vin-export");
const vinDetails = document.getElementById("vin-container");

vinExportButton.addEventListener("click", async () => {
  const rows = Array.from(vinDetails.childNodes).filter((card) => card.nodeType === 1);
  if (vinExportButton.disabled || rows.length === 0) return;
  const isCurrent = () => {
    const current = Array.from(vinDetails.childNodes).filter((card) => card.nodeType === 1);
    return !vinExportButton.disabled && current.length === rows.length &&
      current.every((card, index) => card === rows[index]);
  };
  const vinObj = Object.create(null);
  for (const card of rows) {
    const key = card.querySelector("h3")?.textContent.replaceAll(" ", "_").toLowerCase();
    const value = card.querySelector(".value")?.textContent.trim();
    if (!key || !value) return;
    vinObj[key] = value;
  }
  if (!vinObj.vin) return;

  try {
    await saveJsonExport({
      data: vinObj,
      defaultPath: "vin_details.json",
      save,
      writeTextFile,
      isCurrent,
    });
  } catch (error) {
    addNotification("EXPORT FAILED", "Could not save VIN details.");
    console.error("JSON export failed", error);
  }
});

const input = document.getElementById("terminal-input");

input.addEventListener("input", (e) => {
  if (!input.value.startsWith("> ")) {
    input.value = "> ";
  }
});

input.addEventListener("keydown", (e) => {
  if (input.selectionStart < 2) {
    e.preventDefault();
  }
});

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    if (input.value.trim() === "") {
      return;
    }

    const command = input.value.slice(2).trim();
    if (command.length > 0) {
      appendTerminalOutput(command);
      emit("terminal-command", command);
      input.value = "> ";
    }
    e.preventDefault();
  }
});

const addPidsButton = document.getElementById("pid-add");
addPidsButton.addEventListener("click", () => {
  addCustomPIDRow();
});
