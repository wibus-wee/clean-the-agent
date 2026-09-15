import { readFileSync } from "node:fs";
import { defineConfig } from "@quickgui/cli";

const manifest = readFileSync(new URL("./Cargo.toml", import.meta.url), "utf8");
const version = manifest.match(/^version = "([^"]+)"$/m)?.[1];
if (!version) throw new Error("Cargo.toml must declare the GUI package version");

export default defineConfig({
  language: "rust",
  name: "Clean the Agent",
  identifier: "dev.cleantheagent.app",
  version,
  entry: ".",
  icon: "resources/app/icon.png",
  resources: [
    "resources/licenses/orca-MIT.txt",
    "resources/licenses/lucide-LICENSE.txt",
  ],
  fonts: [
    "node_modules/geist/dist/fonts/geist-sans/Geist-Variable.ttf",
    "node_modules/geist/dist/fonts/geist-mono/GeistMono-Variable.ttf",
  ],
  updates: {
    baseUrl: "https://github.com/wibus-wee/clean-the-agent/releases/latest/download",
  },
});
