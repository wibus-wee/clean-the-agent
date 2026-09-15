# Application icon

| File | Purpose |
| --- | --- |
| `icon-raw.png` | Original 1254 × 1254 artwork supplied for Clean the Agent. |
| `icon.png` | Production 1024 × 1024 PNG consumed by QuickGUI packaging. |

The production image preserves the source composition and alpha channel while
normalizing its transparent outer edge for macOS icon generation. QuickGUI
creates the packaged `.icns` and platform sizes from `icon.png`.
