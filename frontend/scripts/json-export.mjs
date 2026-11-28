// Tauri 2 JSON exports: explicit picker or one fixed app-data file.
// The actual permission boundary is the Tauri filesystem scope, not this helper.
export async function saveJsonExport({
  data, defaultPath, save, writeTextFile, automatic = false, appDataBaseDir,
  isCurrent = () => true,
}) {
  const contents = JSON.stringify(data, null, 2);
  if (automatic) {
    if (defaultPath !== "dtc_log.json" || appDataBaseDir == null) {
      throw new Error("Unsupported automatic export destination");
    }
    if (!isCurrent()) return false;
    await writeTextFile("dtc_log.json", contents, { baseDir: appDataBaseDir });
    return true;
  }

  const path = await save({
    title: "Save as JSON",
    defaultPath,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path || !isCurrent()) return false;
  await writeTextFile(path, contents);
  return true;
}
