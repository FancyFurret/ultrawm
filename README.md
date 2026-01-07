# UltraWM

> [!WARNING]
> **This project is in early alpha.** It is very prone to bugs, incomplete features, and breaking changes. It is **not intended for daily use yet**. Use at your own risk, and expect things to break!

**UltraWM** is a next-generation, cross-platform tiling window manager written in Rust. Designed for power users with
ultrawide or multi-monitor setups, UltraWM provides flexible window tiling with support for grouped workspaces at any
level of your layout. Tile windows seamlessly, merge multiple monitors into one logical display, and maintain a
consistent tiling experience across operating systems. UltraWM supports animated previews, flexible layouts, and is
built to be highly configurable and extensible. It features robust mouse-driven window management—drag, resize, and tile
windows directly with the mouse—while also supporting (and planning to expand) keybind/hotkey controls. Currently
supports Windows and macOS, with Linux support planned.

---

## Features

- **Cross-platform**: Windows and macOS supported (Linux planned)
- **Ultrawide & Multi-Monitor Support**: Merge multiple monitors into one logical display
- **Groups & Workspaces**: Create workspace groups anywhere in your layout—globally or per-region
- **Excellent Mouse Support**: Drag, resize, and tile windows visually
- **Keybinds/Hotkeys**: Keyboard shortcuts for window management
- **Tiling Layouts**: Automatic, non-overlapping window tiling
- **Animated Tile Previews**: Smooth, configurable visual feedback
- **Configurable Gaps**: Set gaps between windows and screen edges

---

## Core Concepts

### Groups

A **Group** is a container that holds multiple **workspaces** and shows one at a time. Groups can be created anywhere
your layout allows—at the root level for global workspace switching, or nested within your layout for regional
workspaces.

### Workspaces

A **Workspace** belongs to a group and contains windows arranged by a **layout** (e.g., Container Tree). Each workspace
in a group can have completely different windows and arrangements.

### Layouts

A **Layout** determines how windows are arranged within a workspace. The default is **Container Tree**, which allows
flexible, nested horizontal and vertical splits.

### The Hierarchy

```
Display (possibly merged monitors)
└── Root Group
    ├── Workspace "Default"
    │   └── Layout (ContainerTree)
    │       ├── Window A
    │       ├── Window B
    │       └── Nested Group
    │           ├── Workspace "Web"
    │           └── Workspace "Dev"
    └── Workspace "Personal" (optional additional global workspace)
        └── Layout (ContainerTree)
            └── ...
```

---

## Usage Patterns

### Simple (No Workspaces)

If you don't care about workspaces, you just have windows in a layout. The root group has one workspace and is
effectively invisible.

### Global Workspaces

Add multiple workspaces to the root group. Switching workspaces swaps your entire screen.

### Regional Workspaces

Create nested groups within your layout. Each region can switch workspaces independently—perfect for ultrawide setups
where you want the left half and right half to have separate workspace groups.

### Tabbed Windows

Create a group containing single windows instead of full layouts. This gives you tabbed/grouped windows in one spot.

---

## Layouts

- **Container Tree**: The default layout, allowing flexible, nested splits and arrangements of windows—well-suited for
  ultrawide and complex workflows.
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

## Developing

Before committing, please run and ensure `cargo tidy` passes to reformat and fix issues.

## Feature Status

### Platform Support

- [x] macOS
- [x] Windows
- [ ] Linux

### Core

- [x] Container Tree layout (nested splits)
- [x] Mouse-driven tiling & resizing
- [x] Animated tile previews
- [x] Layout persistence across restarts
- [x] AI-assisted layout suggestions
- [ ] Groups & Workspaces
- [ ] Keybinds/Hotkeys
- [ ] Monitor merging
- [ ] Window rules