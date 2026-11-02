import init, { get_state } from "./pkg/jagguar_wasm.js";

const button = document.getElementById("get-state");
const output = document.getElementById("output");

function show(message) {
  output.textContent = message;
}

try {
  await init();
  button.disabled = false;
  show("Ready.");
} catch (error) {
  show(`Failed to load wasm: ${error}`);
}

button.addEventListener("click", async () => {
  button.disabled = true;
  show("Requesting state…");
  try {
    show(await get_state());
  } catch (error) {
    show(`Error: ${error}`);
  } finally {
    button.disabled = false;
  }
});
