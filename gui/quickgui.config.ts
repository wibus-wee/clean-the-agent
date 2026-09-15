import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  language: "rust",
  name: "Clean the Agent",
  identifier: "dev.cleantheagent.app",
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
});
