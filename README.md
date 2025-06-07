# UltraWM

**UltraWM** is a next-generation, cross-platform tiling window manager written in Rust. Designed for power users with ultrawide or multi-monitor setups, UltraWM enables you to partition your physical displays into multiple virtual monitors, tile windows seamlessly across monitor borders (including picture-by-picture setups), and maintain a consistent, advanced tiling experience across operating systems. UltraWM supports animated previews, flexible layouts, and is built to be highly configurable and extensible. It features robust mouse-driven window management—drag, resize, and tile windows directly with the mouse—while also supporting (and planning to expand) keybind/hotkey controls. While it currently supports Windows and macOS, Linux support is planned.

---

## Features

- **Cross-platform**: Windows and macOS supported (Linux planned)
- **Ultrawide & Multi-Monitor Partitioning**: Split your display into multiple virtual monitors
- **Excellent Mouse Support**: Drag, resize, and tile windows visually
- **Keybinds/Hotkeys**: Keyboard shortcuts for window management
- **Tiling Layouts**: Automatic, non-overlapping window tiling
- **Animated Tile Previews**: Smooth, configurable visual feedback
- **Workspaces**: Multiple desktops per partition/virtual display
- **Configurable Gaps**: Set gaps between windows and screen edges

---

## Terminology

- **Partition**: A "virtual display"—a region of a physical monitor (or the whole monitor) that acts as its own independent screen. Each partition can have multiple workspaces that you can swap between. Each workspace manages windows according to a layout.
- **Workspace**: A virtual desktop within a partition, containing a set of windows and a layout.
- **Layout**: The algorithm that arranges windows within a workspace (e.g., tiling, tabbed, floating).

---

## Layouts

- **Container Tree**: The default layout, allowing flexible, nested splits and arrangements of windows—well-suited for ultrawide and complex workflows.
- _More layouts may be added down the road_

---

## Download

You can download the latest pre-built version of UltraWM from GitHub Actions:

1. Go to the [GitHub Actions page](https://github.com/FancyFurret/ultrawm/actions).
2. Click on the most recent successful workflow run (look for a green checkmark).
3. Scroll down to the "Artifacts" section at the bottom of the run summary.
4. Download the artifact for your platform (e.g., Windows or macOS).
5. Extract the downloaded archive and run the UltraWM executable.

---

## Building from Source

If you prefer to build UltraWM yourself, follow these steps:

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed (use the latest stable version).
2. Clone this repository:
   ```sh
   git clone https://github.com/FancyFurret/ultrawm.git
   cd ultrawm
   ```
3. Build and run UltraWM:
   ```sh
   cargo run
   ```

On first run, UltraWM will use default configuration values.

---

## Roadmap

| Feature/Goal                     | Status     |
| -------------------------------- | ---------- |
| macOS support                    | ✅ Done    |
| Windows support                  | ✅ Done    |
| Linux support                    | ⏳ Planned |
| Container tree layout            | ✅ Done    |
| Hotkeys                          | ⏳ Planned |
| Layout stability/rules/save/load | ⏳ Planned |
| Partitions (virtual displays)    | ⏳ Planned |
| Workspaces                       | ⏳ Planned |
| AI integration                   | ⏳ Planned |

---
