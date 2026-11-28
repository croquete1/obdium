const { listen, emit } = window.__TAURI__.event;
import { replaceReadinessSnapshot, readinessLabels } from "./readiness-state.mjs";
import {
  exportDtcs,
  addGraphDropdownOption,
  appendTerminalOutput,
  addNotification,
  listenExpandPID,
  stopGraphTracking,
  resumeGraphTracking,
} from "./features.js";
import { runConnectionQueries } from "./connection-lifecycle.mjs";
import {
  saveConnectionConfig,
  updateUnitConversionDropdowns,
} from "./settings.js";

const connectionLabel = document.getElementById("connection-label");
const connectionIcon = document.getElementById("connection-icon");
const replayResponses = document.getElementById("replay-responses");
const recordResponses = document.getElementById("record-responses");
const connectButton = document.getElementById("btn-connect");
const disconnectButton = document.getElementById("btn-disconnect");
const demoStatus = document.getElementById("demo-status");

listen("update-card", (event) => {
  const cards = document.querySelectorAll(".card");
  const exists = Array.from(cards).some((card) => {
    return card.textContent.includes(event.payload.name);
  });

  if (window.obdViewPaused) return;

  // update any graphs that might be using this
  emit("update-graphs", {
    name: event.payload.name,
    value: event.payload.value,
    unit: event.payload.unit,
  });

  // create card if it doesnt exist
  if (!exists) {
    const container = document.querySelector(".grid");
    if (!container) return;

    // create the card
    // has a header, value, and then unit
    const card = document.createElement("div");
    const h3 = document.createElement("h3");
    const valueDiv = document.createElement("div");
    const unitSpan = document.createElement("span");

    card.className = "card";
    valueDiv.className = "value";
    unitSpan.className = "unit";

    h3.textContent = event.payload.name;
    unitSpan.textContent =
      event.payload.unit.toLowerCase() === "no data" ? "" : event.payload.unit;

    // if the unit is no data meaning that scalar has 'no data'
    // (see backend docs), then display 'N/A' for value
    const valueText =
      event.payload.unit.toLowerCase() === "no data"
        ? "N/A"
        : event.payload.value.toString();

    // add value and unit to one div to align horizontally
    valueDiv.appendChild(document.createTextNode(valueText + " "));
    valueDiv.appendChild(unitSpan);

    // append elements to card
    card.appendChild(h3);
    card.appendChild(valueDiv);

    container.appendChild(card);

    return;
  }

  cards.forEach((card) => {
    // Don't update card data.
    if (card.classList.contains("dimmed")) {
      return;
    }

    const h3 = card.querySelector("h3");
    if (!h3) return;

    const title = h3.textContent?.toLowerCase();
    if (title == event.payload.name.toLowerCase()) {
      const valueElem = card.querySelector(".value");

      if (valueElem) {
        const textNode = Array.from(valueElem.childNodes).find(
          (node) => node.nodeType === Node.TEXT_NODE,
        );

        if (textNode) {
          textNode.textContent =
            event.payload.unit.toLowerCase() === "no data"
              ? "N/A "
              : event.payload.value.toString() + " ";

          const unitElem = valueElem.querySelector(".unit");
          unitElem.textContent =
            event.payload.unit.toLowerCase() === "no data"
              ? ""
              : event.payload.unit;
        }
      }
    }
  });

});

listen("vehicle-details", (event) => {
  const vin = document.querySelector(".vin");
  const makeModel = document.querySelector(".car-model");

  if (!hideVin) {
    vin.textContent = "VIN: " + event.payload.vin.toUpperCase();
  } else {
    vin.textContent =
      "VIN: " + event.payload.vin.toUpperCase().slice(0, 6) + "*".repeat(12);
  }

  makeModel.textContent = (
    event.payload.make +
    " " +
    event.payload.model
  ).toUpperCase();
});

let connectionGeneration = 0;

listen("connection-status", async (event) => {
  const protocolDropdown = document.getElementById("protocol-menu");
  const protocolSelected = document.getElementById("protocol-selected");
  const status = document.getElementById("connection-details");

  if (event.payload.connected) {
    if (window.connected) return;

    const serialPort = event.payload.serialPort;
    const baudRate = event.payload.baudRate;
    const protocol = event.payload.protocol;

    connectionLabel.textContent =
      "ELM327 CONNECTED VIA " + serialPort.toUpperCase();
    status.textContent =
      "CONNECTED THROUGH SERIAL PORT " + serialPort.toUpperCase();
    connectionIcon.src = "/assets/icons/connected.png";
    window.connected = true;
    replaceReadinessSnapshot([]);
    document.getElementById("readiness-tests-list").replaceChildren();
    document.getElementById("dtc-list").replaceChildren();
    document.getElementById("dtc-header").textContent = "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)";
    const generation = ++connectionGeneration;
    resumeGraphTracking();

    // Features only accessible in real mode
    if (serialPort !== "DEMO MODE") {
      recordResponses.disabled = false;
      replayResponses.disabled = false;
    } else
      // show demo mode status
      demoStatus.style.display = "flex";

    connectButton.disabled = true;
    disconnectButton.disabled = false;

    document.getElementById("baud-rate-selected").textContent = baudRate;
    document.getElementById("serial-port-selected").textContent = serialPort;
    protocolSelected.dataset.value = protocol;
    protocolSelected.textContent = protocolDropdown.querySelector(
      `li[data-value="${protocol}"]`,
    ).textContent;

    window.connectionConfig = {
      serialPort: serialPort,
      baudRate: baudRate,
      protocol: parseInt(protocol),
    };

    addNotification("CONNECTED VIA ELM", event.payload.message);

    saveConnectionConfig();

    if (window.autoCheckCodes) {
      emit("get-dtcs");
    }

    if (window.unitPreferences) {
      emit("set-unit-preferences", window.unitPreferences);
      updateUnitConversionDropdowns();
    }

    const isCurrent = () => window.connected && connectionGeneration === generation;
    try {
      await runConnectionQueries({
        isCurrent,
        delay: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
        emit,
        firstDelay: serialPort === "DEMO MODE" ? 15000 : 7000,
      });
    } catch (error) {
      if (isCurrent()) {
        addNotification("DIAGNOSTIC REQUEST FAILED", "Could not initialize diagnostic readings.");
        console.error("Connection initialization query failed", error);
      }
    }
  } else {
    const wasConnected = window.connected;

    connectionLabel.textContent = "ELM327 NOT CONNECTED";
    status.textContent = "NO CONNECTION ESTABLISHED";
    connectionIcon.src = "/assets/icons/not-connected.png";
    window.connected = false;
    replaceReadinessSnapshot([]);
    document.getElementById("readiness-tests-list").replaceChildren();
    document.getElementById("dtc-list").replaceChildren();
    document.getElementById("dtc-header").textContent = "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)";
    connectionGeneration++;
    stopGraphTracking();
    recordResponses.disabled = true;
    replayResponses.disabled = true;
    connectButton.disabled = false;
    disconnectButton.disabled = true;

    addNotification(
      wasConnected ? "DISCONNECTED FROM ELM" : "CONNECTION NOT ESTABLISHED",
      event.payload.message,
    );

    if (demoStatus.style.display !== "none") demoStatus.style.display = "none";
  }
});

listen("update-pids", (event) => {
  const pids = event.payload;
  const pidList = document.getElementById("pid-list");
  pidList.innerHTML = ``;

  for (const pidInfo of pids) {
    const pidGroup = document.createElement("div");
    pidGroup.className = "pid-group";
    pidGroup.innerHTML = `
        <div class="pid-container">
            <div class="info-row">
            <button class="arrow-icon"><img src="/assets/icons/arrow-icon.png"></button>
            <span class="name"></span>
            </div>
            <div class="pid-details" style="display: none; height: 0;">
            <div class="pid-data-columns">
                <div class="pid-column">
                <div class="pid-label">MODE</div>
                <div class="pid-value"></div>
                </div>
                <div class="pid-column">
                <div class="pid-label">PID</div>
                <div class="pid-value"></div>
                </div>
                <div class="pid-column">
                <div class="pid-label">COMMAND</div>
                <div class="pid-value"></div>
                </div>
                <div class="pid-column">
                <div class="pid-label">EQUATION</div>
                <div class="pid-value"></div>
                </div>
                <div class="pid-column">
                <div class="pid-label">UNIT</div>
                <div class="pid-value"></div>
                </div>
            </div>
            <button class="pid-button" id="remove-pid">REMOVE</button>
            </div>
        </div>
        `;

    if (!pidInfo.supported) {
      pidGroup.querySelector(".pid-container").style.opacity = "0.15";
    }
    pidGroup.querySelector(".info-row .name").textContent =
      (pidInfo.supported ? "[SUPPORTED]" : "[UNSUPPORTED]") +
      "  " + String(pidInfo.pidName ?? "").toUpperCase();
    const values = pidGroup.querySelectorAll(".pid-value");
    const mode = String(pidInfo.mode ?? "");
    const pid = String(pidInfo.pid ?? "");
    values[0].textContent = "$" + mode;
    values[1].textContent = pid;
    values[2].textContent = mode + pid;
    values[3].textContent = String(pidInfo.formula || "??");
    values[4].textContent = String(pidInfo.unit || "??").toUpperCase();

    pidList.appendChild(pidGroup);

    listenExpandPID(pidGroup);

    addGraphDropdownOption(
      pidInfo.pid,
      pidInfo.pidName,
      pidInfo.unit,
      pidInfo.formula,
    );
  }

  // Increment results counter
  const header = document.getElementById("pid-list-header");
  if (header.textContent != "VIEW PIDS (" + pidList.children.length + ")") {
    addNotification("PID INDEX", "Updated list of supported parameter ID's.");
  }

  header.textContent = "VIEW PIDS (" + pidList.children.length + ")";
});

const dtcList = document.getElementById("dtc-list");
const dtcHeader = document.getElementById("dtc-header");

listen("update-dtcs", (event) => {
  if (!window.connected) return;
  const dtcs = event?.payload;
  // Reject malformed snapshots before changing the list or exporting it.
  if (!Array.isArray(dtcs) || dtcs.some((code) =>
    !code || typeof code.name !== "string" || !code.name.trim() ||
    typeof code.category !== "string" ||
    typeof code.description !== "string" ||
    typeof code.location !== "string" ||
    typeof code.permanant !== "boolean"
  )) {
    dtcList.replaceChildren();
    dtcHeader.textContent = "DIAGNOSTIC TROUBLE CODES (UNAVAILABLE)";
    return;
  }

  dtcList.replaceChildren();
  // The legacy backend also returns an empty list on acquisition failure.
  // Until it sends typed scan outcomes, absence of codes is unverified.
  if (dtcs.length === 0) {
    dtcHeader.textContent = "DIAGNOSTIC TROUBLE CODES (NONE RETURNED — UNVERIFIED)";
    return;
  }
  dtcHeader.textContent = "DIAGNOSTIC TROUBLE CODES (" + dtcs.length + ")";

  for (const troubleCode of dtcs) {
    const description = troubleCode.permanant
      ? troubleCode.description + " [PERMANENT CODE]"
      : troubleCode.description + " [NON-PERMANENT CODE]";

    let dtcRow = document.createElement("div");
    dtcRow.className = "info-row";
    dtcRow.style = "height: 60px; position: relative;";
    dtcRow.innerHTML = `
          <div class="category" id="dtc-category" style="font-size: 40px; font-weight: 900; margin-left: 6px; min-width: 70px; text-align: center; display: inline-block;">
              
          </div>
          <div class="name" style="display: inline-block; vertical-align: top;">
              <span class="name" id="dtc-name" style="font-size: 25px; font-weight: 700;"></span>
              <div class="name" id="dtc-location" style="color: #BDBDBD; font-size: 15px; margin-top: -5px; font-weight: 600;">
              
              </div>
          </div>
          
          <svg xmlns="http://www.w3.org/2000/svg" width="40" height="30" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"
              style="margin-left: 190px; position: absolute; top: 50%; transform: translateY(-50%);">
              <path d="M5 12h14" />
              <path d="m12 5 7 7-7 7" />
          </svg>
          
          <div class="name" id="dtc-description"
              style="position: absolute; color: #BDBDBD; left: 240px; top: 0; bottom: 0; max-width: 600px; overflow-wrap: break-word; display: flex; align-items: center; height: 100%;">
              
          </div>
          `;

    dtcRow.querySelector("#dtc-category").textContent = String(troubleCode.category ?? "");
    dtcRow.querySelector("#dtc-name").textContent = String(troubleCode.name ?? "");
    dtcRow.querySelector("#dtc-location").textContent = String(troubleCode.location ?? "");
    dtcRow.querySelector("#dtc-description").textContent = description;
    dtcList.appendChild(dtcRow);
  }

  if (window.autoSaveCodes) {
    exportDtcs(true);
  }
});

const serialPortMenu = document.getElementById("serial-port-dropdown-menu");
const serialPortSelected = document.getElementById("serial-port-selected");
const baudRateSelected = document.getElementById("baud-rate-selected");

listen("update-serial-ports", (event) => {
  serialPortMenu.innerHTML = "";

  const demoModePortOption = document.createElement("li"); 
  demoModePortOption.textContent = "DEMO MODE";
  demoModePortOption.dataset.value = "DEMO MODE";
  serialPortMenu.appendChild(demoModePortOption);

  demoModePortOption.addEventListener("click", () => {
    baudRateSelected.textContent = "0";
  })

  if (!event.payload) {
    return;
  }
  
  console.log("received serial ports:", event);

  for (const [port, baudRate] of event.payload) {
    // append serial port connect option
    const portOption = document.createElement("li");
    portOption.textContent = port;
    portOption.dataset.value = port;
    serialPortMenu.appendChild(portOption);

    portOption.addEventListener("click", () => {
      baudRateSelected.textContent = baudRate;
    });
  }

  const refreshButton = document.getElementById("refresh-serial-ports");
  const refreshIcon = refreshButton.querySelector("svg");

  refreshIcon.classList.remove("spinning");
});

const readinessTests = document.getElementById("readiness-tests-list");
listen("update-readiness-tests", (event) => {
  if (!window.connected) return;
  readinessTests.replaceChildren();

  const tests = replaceReadinessSnapshot(event.payload);
  addNotification("READINESS TESTS", tests.length
    ? "Updated I/M readiness test information."
    : "No I/M readiness test information is available.");
  for (const test of tests) {
    const testRow = document.createElement("div");
    testRow.className = "info-row";
    testRow.style = "justify-content: space-between;";
    testRow.innerHTML = `
<div class="name" id="test-name" style="flex: 2; font-weight: 700; color: #f7f3ff;"></div>
    <div class="name" id="test-availability" style="flex: 1; font-size: 0.97rem; font-weight: 500; text-transform: uppercase; color: #d4dadf;"></div>
      <div class="name" id="test-completeness" style="flex: 1; font-size: 0.97rem; font-weight: 500; text-transform: uppercase; color: #d4dadf;"></div>

    `;

    const labels = readinessLabels(test);
    testRow.querySelector("#test-name").textContent = "TEST: " + test.name;
    testRow.querySelector("#test-availability").textContent = labels.availability;
    testRow.querySelector("#test-completeness").textContent = labels.completeness;
    readinessTests.appendChild(testRow);
  }
});

listen("update-command-output", (event) => {
  appendTerminalOutput(event.payload);
});

const exportButton = document.getElementById("vin-export");

// show vehicle info if no error
listen("decode-vin-result", (event) => {
  const data = event?.payload;
  const request = window.pendingVinRequest;
  if (!request || data?.request_id !== request.id) return;
  window.pendingVinRequest = null;
  clearTimeout(request.timer);
  const container = document.getElementById("vin-container");
  const status = document.getElementById("vin-request-status");
  if (!container || !status) return;
  const query = document.querySelector(".vin-input")?.value.trim().toUpperCase();

  exportButton.disabled = true;
  container.replaceChildren();
  const now = performance.now();
  if (!Number.isFinite(now) || now < request.started || now >= request.deadline) {
    status.textContent = "VIN request timed out. Try again.";
    return;
  }
  status.textContent = "No VIN details available.";
  // Even a correlated response must match the requested VIN if present.
  if (typeof data?.vin === "string" && data.vin.trim() &&
      data.vin.trim().toUpperCase() !== query) return;
  addNotification("VIN DECODING", typeof data?.error_msg === "string"
    ? data.error_msg : "No VIN details available.");
  if (typeof data?.vin !== "string" || !data.vin.trim()) return;

  // show only available string fields
  for (const [key, value] of Object.entries(data)) {
    if (key === "error_msg" || key === "request_id" || typeof value !== "string" ||
        !value.trim() || value === "N/A" || value === "-1") continue;

    const name = key.replaceAll("_", " ").toUpperCase();

    const card = document.createElement("div");
    const h3 = document.createElement("h3");
    const valueDiv = document.createElement("div");

    card.className = "card";
    valueDiv.className = "value";

    h3.textContent = name;
    valueDiv.textContent = value.toUpperCase();
    valueDiv.style.fontSize = "1.3rem";
    valueDiv.style.wordBreak = "break-word";
    valueDiv.style.whiteSpace = "normal";
    valueDiv.style.maxWidth = "320px";

    // append elements to card
    card.appendChild(h3);
    card.appendChild(valueDiv);

    container.appendChild(card);
  }

  exportButton.disabled = false;
  status.textContent = "VIN details available.";
});

listen("display-notification", (event) => {
  const title = event.payload.title;
  const desc = event.payload.description;
  console.log(title, desc);
  
  addNotification(title, desc);
});