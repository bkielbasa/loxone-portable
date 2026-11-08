# Stage 1: Workspace, Navigation, 4-Mode Project Tree & Migration UX — Design Spec

## Goal
Recreate the workspace shell, dockable layout, 4-mode `Project Trees` selector, bottom sheet tab bar (`Afx:TabWnd`), and seamless Loxone Config migration (`My Miniservers` / `MiniserverKeychain` + keyboard shortcuts) of **Loxone Config 17 (`17.2.10.21`)** inside `loxone-portable`, while preserving our dark schematic canvas (`#12151B`).

## Architecture & Components

### 1. Top Shell (`CLxTitleBar` + `CMiniserverStateWidget` + `CRibbonWidget` + `QuickSearch`)
- **Top Title Bar (`CLxTitleBar`)**:
  - Left: Hamburger Main Menu (`≡`) with file/project actions (`New Project`, `Open Project…`, `Save`, `Save As…`, `Export SVG`, `Export HTML Documentation`) + Quick Access toolbar (`New`, `Open`, `Save`, `Undo`, `Redo`).
  - Center-Left: `🏠 Home` tab (`CHomeButton`) switching to the project launcher screen, and the active project document tab (`📄 <Project Title> ✕`) with dirty indicator and close button.
  - Right: Dock layout toggle (`Inspector on Left / Right`), Message Center / Diagnostics button, and Language selector (`EN` / `DE` / `PL`).
- **Left Miniserver State Tile (`CMiniserverStateWidget`)**:
  - Sits on the left side of the ribbon row (`CCaptionBar`).
  - Displays a prominent `Connect to Miniserver` button (with live status indicator and host/project info) above three quick action buttons: `Search` (Network Search), `My Miniservers` (opens `MiniserverKeychain` view), and `Manual` (opens manual connection view).
- **Ribbon (`CRibbonWidget`) & QuickSearch Bar**:
  - Starts with a green magnifying-glass button (`🔍 QuickSearch`, `Ctrl+F` / `Cmd+F`) that toggles a wide live search bar directly above the schematic canvas with instant filtering across blocks, pages, and periphery items.
  - 5 core ribbon tabs (`Devices`, `Function Blocks`, `Tools`, `Testing`, `Diagnostics`) + contextual object/live tabs.
  - Right side of ribbon: `Web Interface` button opening `http://<miniserver_host>/` in the system browser.

### 2. Dock Layout (`Properties` + `Project Trees`) & Inspector Enhancements
- **Default Layout (`InspectorDockSide::LeftBesideTree`)**:
  - Matches Loxone Config 17 (`CPropertyWindowDockWidget` at `x=0..280`, `CTreeDockWidget` at `x=280..560`, schematic canvas on the right).
  - Includes a header toggle button (`⇄`) in the `Properties` title bar and top bar to switch to `InspectorDockSide::RightSplit` (`Project Trees` on the left, `Properties` on the right) for widescreen ergonomics, plus pin/collapse (`📌`) buttons on both dock headers.
- **Inspector Enhancements (`CQtPropWindow`)**:
  - Dynamic title bar (`Properties (Page)`, `Properties (<Type>)`).
  - `CSettingsHeader` with object title, inline property filter (`Filter`), and quick-action buttons.
  - 9-dot color palette (`Color`) and 10-star rating selector (`Rating` `★☆☆☆☆☆☆☆☆☆`) for blocks and categories.

### 3. 4-Mode `Project Trees` (`CTreeWindow` / `LxTreeCtrl`)
- Top dropdown selector (`ComboBox`) switching between Loxone Config 17's 4 tree modes (with backward-compatible mapping to `LeftTab`):
  1. **`Periphery` (`LeftTab::Periphery`)**:
     - Global project sections at the top (`Rooms`, `Categories`, `Icons`, `Users & Permissions`, `Managed Tablets`, `Constants`, `Memory Flags`, `System Variables`, `Operating Modes`, `Operating Times`, `Time Functions`, `Remote Controls`, `Push Notifications`, `Weather Server`, `Network Devices`, `Audio`, `Trusts`, `System Status`, `Device Monitor`, `AutoPilot Rules`) followed by the Miniserver (`LoxLIVE`) and all hardware buses (`Loxone Link`, `Loxone Tree`, `Loxone Air`, `Network`).
  2. **`Configuration` (`LeftTab::Pages`)**:
     - Hierarchical list of all schematic `Pages`, where each page has an expandable `[+]` node listing every function block on that page with its `(Room, Category)` subtitle in parentheses.
  3. **`Rooms` (`LeftTab::Rooms`)**:
     - Grouped by Room (`Place`) and Category (`Category`), showing assigned blocks and periphery I/O under each room.
  4. **`Devices` (`LeftTab::System` / `Devices`)**:
     - Dedicated hardware device hierarchy (Miniserver, Extensions, Tree branch devices, Air devices, Network devices) with serial numbers, bus types, and `🔦 Identify` / `🔢 Serials` actions.

### 4. Bottom Sheet Tabs (`Afx:TabWnd`) & Seamless Migration (`MiniserverKeychain`)
- **Bottom Sheet Tab Bar (`Afx:TabWnd`)**:
  - Docked at the bottom of the schematic canvas (`CentralPanel`).
  - Left controls: `|<` (first page), `<` (previous page), `>` (next page), `>|` (last page), `☰` (searchable popup menu of all pages), `+` (create new page).
  - Horizontally scrollable angled/trapezoid sheet tabs with status/category colored dot, active tab highlight, double-click inline rename, and right-click context menu (`Inspect Page`, `Rename Page`, `Duplicate Page`, `Delete Page`).
- **Seamless Loxone Config Migration (`My Miniservers` / `MiniserverKeychain`)**:
  - `SavedMiniserverEntry` (`name`, `project_name`, `user`, `serial`, `password`, `internal_address`, `external_address`, `save_locally`) and `MiniserverKeychain` persisted in `~/.config/loxone-portable/miniservers.json`.
  - Automatic extraction of Miniserver metadata (`Title`, `Serial`, `IntAddr`, `ExtAddr`, `HoN`) from `<C Type="LoxLIVE">` whenever any `.Loxone` project is opened or discovered, automatically populating `My Miniservers` without manual re-entry.
  - Tabbed/modeled `MiniserverModalState` supporting `My Miniservers` table + detail editor and `Connect to Miniserver` manual connection dialog.
- **Keyboard Shortcuts**:
  - `F5`: Toggle offline simulation / live view.
  - `Ctrl+F` / `Cmd+F`: Toggle the `QuickSearch` bar above the canvas.
  - `Ctrl+PageUp` / `Ctrl+PageDown`: Switch to previous / next schematic page tab.
  - `Ctrl+Shift+S` / `Cmd+Shift+S`: Save Project As (`.Loxone`).
