# Pak Assets Manager - Codebase Documentation

## Project Overview

**Pak Assets Manager** is a GTK4-based Rust application for browsing, viewing, and editing Chrome/Brave browser `.pak` resource files. It supports multiple pak file formats (Version 4, 5, and legacy) with tabbed interface for simultaneous file management.

---

## Architecture

```
pak-assets-manager/
├── src/
│   ├── main.rs              # Application entry point
│   ├── pak/                 # Pak file format handling
│   │   ├── mod.rs           # Core data structures
│   │   ├── parser.rs        # File parsing (V4, V5, legacy)
│   │   └── writer.rs        # File serialization
│   ├── browser_detector.rs  # Brave browser auto-detection
│   ├── ui/                  # GTK4 user interface
│   │   ├── window.rs        # Main window with tabs
│   │   ├── resource_list.rs # Resource sidebar
│   │   └── editor_canvas.rs # Image editor widget
│   ├── editor/              # Drawing/editing tools
│   │   ├── brush.rs         # Brush/eraser tools
│   │   └── history.rs       # Undo/redo system
│   └── image_processor/     # Image manipulation
│       ├── resizer.rs       # Resize operations
│       └── optimizer.rs     # PNG optimization
```

---

## Module Breakdown

### 1. Entry Point (`main.rs`)

**Purpose**: Application initialization and lifecycle management

**Key Components**:
- `APP_ID`: GTK4 application identifier (`com.example.PakAssetsManager`)
- `MAIN_WINDOW`: Thread-local storage to prevent Rc drop
- `main()`: Initializes GTK4, creates Application instance
- `build_ui()`: Constructs MainWindow, stores in thread-local

**Execution Flow**:
```rust
main()
  └── app.run()
      └── build_ui(app)
          └── MainWindow::new(app)
              └── Store in MAIN_WINDOW thread_local
              └── window.present()
```

---

### 2. Pak Module (`pak/`)

#### 2.1 Core Data Structures (`pak/mod.rs`)

**Resource**: Individual resource entry
```rust
pub struct Resource {
    pub id: u16,              // Resource identifier
    pub offset: u32,          // File offset
    pub data: Vec<u8>,        // Raw binary data
    pub width: Option<u32>,   // Parsed image width
    pub height: Option<u32>, // Parsed image height
    pub format: Option<ImageFormat>, // PNG/WebP/Unknown
}
```

**PakFile**: Complete pak file container
```rust
pub struct PakFile {
    pub version: u32,         // Format version (4, 5, etc.)
    pub encoding: u8,         // Text encoding
    pub resources: Vec<Resource>,
    pub aliases: Vec<Alias>,
}
```

**Key Methods**:
- `load(path)`: Parse pak file from disk
- `save(path)`: Write pak file to disk
- `get_image(id)`: Decode resource to DynamicImage
- `replace_resource(id, data)`: Update resource data
- `get_image_resources()`: Filter image-type resources

**Image Format Detection**:
- PNG: Magic bytes `89 50 4E 47 0D 0A 1A 0A`
- WebP: `RIFF....WEBP` pattern
- Dimensions parsed from IHDR (PNG) or VP8/VP8X (WebP)

#### 2.2 Parser (`pak/parser.rs`)

**Supported Formats**:

**Version 4 Format**:
```
[0-3]   version (u32) = 4
[4-7]   num_resources (u32)
[8]     encoding (u8)
[9+]    resource entries (num_resources + 1 entries)
        Each entry: id (u16) + offset (u32) = 6 bytes
```

**Version 5 Format**:
```
[0-3]   version (u32) = 5
[4]     encoding (u8)
[5-7]   padding (3 bytes)
[8-9]   num_resources (u16)
[10-11] num_aliases (u16)
[12+]   resource entries (num_resources + 1 entries, 6 bytes each)
[...]   alias entries (num_aliases entries, 4 bytes each)
```

**Parsing Algorithm**:
1. Read version from first 4 bytes
2. Branch by version number
3. Read header fields (resource count, aliases, encoding)
4. Read (num_resources + 1) entries (last is sentinel)
5. Calculate resource sizes from offset deltas
6. Read actual resource data from file
7. Parse image dimensions for visual resources

**Key Functions**:
- `parse_pak_file(path)`: Main entry point
- `parse_version4()`: V4 format handler
- `parse_version5()`: V5 format handler
- `parse_version_legacy()`: V1-3 format handler
- `parse_data_pack()`: Fallback for edge cases

#### 2.3 Writer (`pak/writer.rs`)

**Serialization Process**:
1. Write version header
2. Write resource/alias counts
3. Write resource table (sorted by ID)
4. Write alias table
5. Write actual resource data
6. Handle 4-byte alignment padding

---

### 3. Browser Detector (`browser_detector.rs`)

**Purpose**: Auto-detect Brave browser installations and locate pak files

**Detection Strategy**:

**Linux Paths**:
- `/opt/brave-bin` (AUR/Manual)
- `/var/lib/flatpak/app/com.brave.Browser/...`
- `~/.var/app/com.brave.Browser/config/brave` (User Flatpak)
- `/usr/lib/brave-browser` (System)
- `/snap/brave/current/...` (Snap)
- `/opt/brave`, `/usr/share/brave`

**macOS Paths**:
- `~/Applications/Brave Browser.app/...`
- `/Applications/Brave Browser.app/...`

**Windows Paths**:
- `C:\Program Files\BraveSoftware\Brave-Browser\Application`
- `%LOCALAPPDATA%\BraveSoftware\Brave-Browser\Application`

**Key Functions**:
- `detect_browsers()`: Scan all platforms, return BrowserInfo vector
- `get_brave_install_paths()`: Platform-specific path list
- `scan_for_pak_files(dir)`: Recursive .pak file discovery
- `get_pak_display_name(path)`: Extract filename without extension

---

### 4. UI Module (`ui/`)

#### 4.1 Main Window (`ui/window.rs`)

**Architecture**: Tabbed interface with notebook widget

**MainWindow Structure**:
```rust
pub struct MainWindow {
    window: ApplicationWindow,           // GTK4 root window
    tabs: RefCell<HashMap<u32, PakTab>>, // Page num -> tab data
    notebook: Notebook,                  // Tab container
    status_bar: Statusbar,               // Bottom status display
    current_file_label: Label,          // Header title
    tab_counter: RefCell<u32>,          // Unique tab IDs
    self_weak: RefCell<Weak<Self>>,    // Self-reference for callbacks
}
```

**PakTab Structure** (per-tab data):
```rust
struct PakTab {
    pak_file: PakFile,              // Parsed pak data
    path: PathBuf,                  // Source file path
    resource_list: Rc<ResourceList>, // Sidebar widget
    editor_canvas: Rc<EditorCanvas>,   // Editor widget
}
```

**User Flow**:
```
1. Click "Scan Brave"
   └── show_file_browser()
       └── detect_browsers()
       └── show_file_selection_dialog()
           └── Display checkboxes for all 73 pak files
           └── Main files pre-selected, locales unchecked

2. User selects files → Click "Load Selected"
   └── For each selected path:
       └── load_pak_file(path)
           └── PakFile::load(path)
           └── create_tab(pak, path)
               └── Create ResourceList + EditorCanvas
               └── Add to notebook with close button
               └── Setup resource selection callback
```

**Key Methods**:
- `new(app)`: Constructor, setup GTK widgets
- `setup_signals()`: Connect button click handlers
- `show_file_browser()`: Initiate browser scan
- `show_file_selection_dialog()`: File picker with checkboxes
- `load_pak_file()`: Parse and load single pak
- `create_tab()`: Build tab UI components
- `close_tab()`: Remove tab, cleanup resources
- `on_tab_switched()`: Update header on tab change

#### 4.2 Resource List (`ui/resource_list.rs`)

**Purpose**: Sidebar widget showing pak resources

**Features**:
- GTK ListBox with resource entries
- Displays: ID, dimensions, format
- Click to select resource
- Callback system for selection events

**Key Methods**:
- `new()`: Create ListBox in ScrolledWindow
- `set_resources()`: Populate with Resource vector
- `on_selected()`: Set selection callback
- `setup_signals()`: Handle ListBox row selection

#### 4.3 Editor Canvas (`ui/editor_canvas.rs`)

**Purpose**: Image editing surface with drawing tools

**Structure**:
```rust
pub struct EditorCanvas {
    container: Box,                      // Root widget
    drawing_area: DrawingArea,           // Drawing surface
    base_surface: RefCell<Option<ImageSurface>>, // Base image
    overlay_surface: RefCell<Option<ImageSurface>>, // Preview
    current_tool: Rc<RefCell<Tool>>,     // Brush/Eraser
    history: Rc<RefCell<History>>,        // Undo/redo
    last_pos: RefCell<Option<(f64, f64)>>, // Last mouse pos
}
```

**Features**:
- Display decoded PNG/WebP images
- Brush tool with color/size
- Eraser tool
- Undo/redo system
- Cairo-based rendering

**Key Methods**:
- `new()`: Create canvas with gestures
- `set_base_image()`: Load image for editing
- `setup_drawing()`: Connect mouse events
- `undo()` / `redo()`: History navigation
- `clear()`: Reset canvas

---

### 5. Editor Module (`editor/`)

#### 5.1 Brush Tool (`editor/brush.rs`)

**Tool Enum**:
```rust
pub enum Tool {
    Brush { color: (f64, f64, f64), size: f64 },
    Eraser { size: f64 },
}
```

**Brush Struct**: Drawing parameters (size, color)
**Eraser Struct**: Eraser parameters (size)

**Methods**:
- `draw()`: Render stroke on Cairo context
- `set_color()` / `set_size()`: Modify parameters

#### 5.2 History (`editor/history.rs`)

**Purpose**: Undo/redo state management

**Approach**: Store raw pixel data (RGBA bytes) for each snapshot

**HistorySnapshot**:
```rust
struct HistorySnapshot {
    width: i32,
    height: i32,
    data: Vec<u8>, // Raw pixel data
}
```

**History Struct**:
```rust
pub struct History {
    snapshots: Vec<HistorySnapshot>,
    current_index: usize,
    max_snapshots: usize, // Default: 50
}
```

**Key Methods**:
- `push()`: Save current state
- `undo()`: Restore previous state
- `redo()`: Restore next state
- `can_undo()` / `can_redo()`: Query availability
- `clear()`: Reset history

---

### 6. Image Processor (`image_processor/`)

#### 6.1 Resizer (`resizer.rs`)

**Functions**:
- `resize_to_exact_fit()`: Scale to exact dimensions
- `center_crop_or_pad()`: Crop or pad to target size
- `resize_to_fit()`: Lanczos3 resizing

#### 6.2 Optimizer (`optimizer.rs`)

**Purpose**: PNG size optimization for pak file constraints

**Functions**:
- `encode_png()`: PNG encoding
- `optimize_to_exact_size()`: Binary search for target size
- `pad_to_size()`: Add padding chunks
- `calculate_crc32()`: CRC calculation
- `reduce_colors_and_encode()`: Color palette reduction

---

## Execution Map

### Application Startup

```
main()
├── Initialize GTK4
├── Create Application (APP_ID)
├── Connect activate signal → build_ui()
└── app.run() (event loop)

build_ui()
├── MainWindow::new(app)
│   ├── Create ApplicationWindow
│   ├── Setup HeaderBar (Scan Brave button)
│   ├── Create Notebook (tabs)
│   ├── Create Statusbar
│   ├── Setup signal handlers
│   └── Store self_weak reference
├── Store in MAIN_WINDOW thread_local
└── window.present()
```

### Loading Pak Files

```
User clicks "Scan Brave"
└── show_file_browser()
    ├── detect_browsers()
    │   ├── get_brave_install_paths() → Platform paths
    │   └── scan_for_pak_files() → Recursive .pak discovery
    └── show_file_selection_dialog()
        ├── Create Dialog with checkboxes
        ├── Pre-select main files, skip locales
        └── Connect "Load Selected" response

User clicks "Load Selected"
└── For each selected file:
    └── load_pak_file(path)
        ├── PakFile::load(path)
        │   └── parser::parse_pak_file(path)
        │       ├── Read version
        │       ├── Branch to parse_version4/5/legacy
        │       ├── Read resource entries
        │       ├── Read resource data
        │       └── Parse image dimensions
        └── create_tab(pak, path)
            ├── Create ResourceList + EditorCanvas
            ├── Create Paned layout (sidebar + editor)
            ├── Add tab to Notebook
            └── Connect selection callback
```

### Resource Selection & Display

```
User clicks resource in sidebar
└── resource_list.on_selected callback
    └── editor_canvas.set_base_image()
        ├── Decode PNG/WebP from resource.data
        ├── Create Cairo ImageSurface
        └── Queue draw widget
```

### Tab Management

```
User switches tab
└── notebook.connect_switch_page
    └── on_tab_switched(page_num)
        └── Update current_file_label

User closes tab
└── close_btn.connect_clicked
    └── close_tab(page_num)
        ├── Remove from notebook
        ├── Remove from tabs HashMap
        └── If empty, reset label
```

---

## File Format Reference

### Chrome/Brave Pak Format (Version 5)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | version | File format version (5) |
| 4 | 1 | encoding | Text encoding identifier |
| 5 | 3 | padding | Alignment padding (zeros) |
| 8 | 2 | num_resources | Resource entry count |
| 10 | 2 | num_aliases | Alias entry count |
| 12 | 6×(n+1) | entries | Resource entries (id: u16, offset: u32) |
| ... | 4×m | aliases | Alias entries (id: u16, resource_idx: u16) |
| ... | ... | data | Resource binary data |

**Resource Entry** (6 bytes):
- `id` (u16): Resource identifier
- `offset` (u32): Absolute file offset to data

**Resource Size Calculation**:
`size = next_offset - current_offset`

---

## Dependencies

```toml
[dependencies]
gtk4 = { version = "0.8", features = ["v4_12"] }  # UI framework
glib = "0.19"                                      # GTK runtime
libadwaita = "0.6"                                 # Modern GTK widgets
image = { version = "0.25", features = ["png", "webp"] }  # Image decoding
png = "0.17"                                       # PNG encoding
cairo-rs = "0.19"                                  # 2D graphics
anyhow = "1.0"                                     # Error handling
thiserror = "1.0"                                  # Custom errors
byteorder = "1.5"                                  # Binary I/O
dirs = "5.0"                                       # Platform directories
```

---

## Build & Run

```bash
# Build
cargo build

# Run with debug output
cargo run

# Build release
cargo build --release
```

---

## Debug Features

The application includes extensive debug logging:

```rust
[DEBUG] Scan Brave button clicked!
[DEBUG] Found {} browser(s)
[DEBUG] Total pak files found: {}
[DEBUG] User selected {} files to load
[DEBUG] Loading pak file: {:?}
[PAK DEBUG] File: {:?}, Version: {}
[PAK DEBUG] V5: num_resources={}, num_aliases={}
[PAK DEBUG] V5 Entry {}: id={}, offset={}
[PAK DEBUG] V5: Loaded {} resources, {} aliases
[DEBUG] Pak loaded: {} resources
[DEBUG] Creating tab for {} ({} resources, {} images)
```

---

## Memory Management

- **Thread-local window storage**: Prevents MainWindow drop
- **Rc<RefCell> pattern**: Shared ownership for GTK widgets
- **Weak references**: Avoid circular references in callbacks
- **Per-tab isolation**: Each tab owns its PakFile and widgets
- **Resource cleanup**: Tab close removes all associated data

---

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | ✅ Full | Primary development platform |
| macOS | ⚠️ Partial | Path detection implemented, not tested |
| Windows | ⚠️ Partial | Path detection implemented, not tested |

---

## Future Enhancements

1. **Save functionality**: Complete write_pak_file implementation
2. **Image editing**: Brush/eraser tools completion
3. **Export**: Save individual resources to disk
4. **Import**: Replace resources with external files
5. **Search**: Find resources by ID or content
6. **Batch operations**: Multi-file edits
7. **Modern dialogs**: Replace deprecated GTK4 dialogs

---

## License

See project LICENSE file

---

## Authors

Development team - see git history for contributors

---

*Generated: April 2026*
*Version: 0.1.0*
*Format: Professional Technical Documentation*
