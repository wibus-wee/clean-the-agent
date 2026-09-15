# Clean the Agent Design Language

Clean the Agent combines Vercel's Geist visual language with macOS interaction
behavior. QuickGUI renders the application surface; macOS owns file panels,
irreversible confirmation sheets, window behavior, and system appearance changes.

## Principles

| Principle | Rule |
| --- | --- |
| Quiet hierarchy | Use typography, spacing, and 1 px separators before adding containers or color. |
| Provider first | Software providers own capability scopes, findings, and actions. Provider identity is structural content, never a metadata badge. |
| Monochrome actions | Reserve the high-contrast foreground/background pair for the primary action. |
| Dense clarity | Use compact 36 px controls and 12–14 px interface type; keep explanatory copy close to the affected object. |
| Semantic color | Use green, amber, and red only for status or consequences, and always pair color with text. |
| Native consequence | Use a macOS sheet only for irreversible cleanup. Reversible settings apply directly and report progress in-window. |
| Observable work | Run scanning and cleanup outside the UI thread, disable impossible actions, and show per-item failures. |
| Meaningful motion | Animate a glyph's semantic layer, such as a slider knob or disabled stroke. Never scale the complete icon for emphasis. |

## Foundations

| Token | Contract |
| --- | --- |
| Canvas | Pure white or black follows the system appearance. |
| Surface | One subtle neutral step from the canvas; hover and selection each move one further step. |
| Border | Neutral 1 px rules define grouping. Hover color must not become a second accent. |
| Spacing | Use QuickGUI's 4 px scale in even multiples; 8, 12, 16, and 24 px are the normal rhythm. |
| Radius | Controls use 10 px, grouped panels use 16 px, and status badges use a pill radius. |
| Type | Bundle Geist Sans for interface text and Geist Mono for paths; use 12 px metadata, 14 px controls and body, 16–20 px section titles, and 32 px product headings. |
| Icon | Use the 24 px Lucide grid for authored vectors. Prefer SF Symbols for static macOS conventions and layered local SVGs when the product owns motion. |
| Primary | Light foreground on near-black in light mode; the polarity reverses in dark mode. |

The palette follows Geist's background, component, border, and text roles. It
does not copy browser CSS values into native dialogs; macOS renders those.

## Components

The primitives live in [`src/design.rs`](./src/design.rs). Business views may
compose them but must not create competing button, badge, panel, or navigation
styles.

| Component | Rust entry point | Use |
| --- | --- | --- |
| Theme | `Theme::from_context` | Resolve light and dark semantic tokens. |
| Sidebar and toolbar | `sidebar`, `toolbar` | Establish persistent navigation and page actions inside the native window frame. |
| Provider identity | `provider_sidebar_row`, `provider_icon` | Identify a provider in navigation and at the root of its content hierarchy. |
| Overview | `CleanerApp::render_overview` | Summarize every scanned provider and route into its findings or preferences without merging their actions. |
| Navigation row | `sidebar_icon_row` | Switch top-level areas with a semantic icon and neutral selection. |
| Icons | `icons::icon`, `system_info_icon` | Retain bundled Lucide SVGs and resolve SF Symbols with a local fallback. |
| Animated icons | `animated_sliders_icon`, `animated_keyboard_icon` | Compose independently transformable SVG layers without whole-glyph scaling. |
| Category tile | `category_tile` | Explain one supported cleanup domain and opt it into or out of the cleanup plan. |
| Panel | `panel` | Group content only when spacing and separators are insufficient. |
| Buttons | `primary_button`, `toolbar_button`, `destructive_button` | Represent primary, supporting, and irreversible actions. |
| Checkbox | `review_checkbox` | Opt into review-gated cleanup without changing the default plan. |
| Setting switch | `setting_switch` | Toggle a reversible preference while keeping its state and consequence visible. |
| Badge | `status_badge` | Show concise state with text and semantic color. |
| Metadata | `detail_group` | Pair a short Title Case label with one value. |
| Settings browser | `render_tweak_row`, `render_tweak_detail` | Keep large preference catalogs navigable with a compact selection list and one stable detail pane. |
| Feedback | `empty_state`, `message_panel`, QuickGUI `ToastManager` and `ToastViewport` | Keep empty and actionable error states in context; use animated, transient window-level feedback for completed work. |

## Interaction Contract

- On macOS, the window uses QuickGUI's `HiddenInset` title-bar style. Content
  extends to the top edge while the native traffic lights remain available;
  only explicit header regions drag the window and controls opt out of dragging.
  A toast portal mounts only around its bottom-right surface and is absent when
  empty, so overlays never cover the toolbar's drag region.
- The Scan & Clean view uses a list-detail layout. Selection never performs an
  action.
- The primary navigation lists software providers before cross-provider tools.
  A provider page begins with its icon, name, scan state, supported platforms,
  and enabled scope count; capability scopes are nested inside that surface.
- Overview is the default page. Scan All refreshes cleanup artifacts and every
  registered preference recipe in one read-only operation. It never turns that
  shared discovery step into a shared apply action: cleanup and tweaks retain
  their own review and action boundaries.
- Cleanup categories remain visible before and after scanning. Each tile names
  the artifacts it covers, reports findings and reclaimable bytes, and controls
  whether that category contributes actions to the plan.
- Scan starts automatically and can be repeated. Choose Folder… opens the
  native directory panel.
- Cleanup stays disabled when the plan is empty or another operation is active.
- Review-required findings remain excluded until the checkbox is selected.
- Cleanup presents a native confirmation sheet immediately before an
  irreversible write. A Codex switch applies directly because the change is
  reversible; its on-state describes the owned behavior, not the raw
  configuration value. Both directions use scan-time content guards, preserve
  unrelated settings, and resolve their progress toast in place.
- Hover motion may translate, rotate, recolor, or fade an independently rendered
  SVG layer. It must not move layout, run while idle, or replace textual state.
  QuickGUI transitions inherit the system Reduce Motion preference.
- An in-progress toast resolves in place when cleanup or a tweak finishes.
  Successful work dismisses automatically; hover pauses the timer, and both a
  close control and focused Escape dismiss it immediately. Partial or failed
  work keeps its per-item result panel in context so a transient summary never
  hides an actionable problem. Toasts enter with a short fade and vertical
  translation, loading glyphs rotate independently, success glyphs draw
  attention with a local fade and translation, and dismissal fades and slides
  the surface before it leaves the tree.
