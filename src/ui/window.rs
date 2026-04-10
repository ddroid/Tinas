use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, HeaderBar, Box, Button,
           Orientation, Paned, Label, Statusbar, Notebook, ScrolledWindow, CheckButton,
           Separator, ToggleButton, Viewport};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::pak::PakFile;
use crate::pak::Resource;
use crate::ui::resource_list::ResourceList;
use crate::ui::editor_canvas::EditorCanvas;
use crate::ui::tool_panel::ToolPanel;
use crate::ui::version_history_dialog::VersionHistoryDialog;
use crate::browser_detector::{detect_browsers, get_pak_display_name};
use crate::image_processor::optimizer::encode_png;
use crate::temp_db::TempDb;

/// Data for a single pak file tab
struct PakTab {
    pak_file: RefCell<PakFile>,
    path: PathBuf,
    resource_list: Rc<ResourceList>,
    editor_canvas: Rc<EditorCanvas>,
    tool_panel: Rc<ToolPanel>,
    current_resource_id: RefCell<Option<u16>>,
}

pub struct MainWindow {
    window: ApplicationWindow,
    tabs: RefCell<HashMap<u32, PakTab>>, // notebook page num -> PakTab
    notebook: Notebook,
    status_bar: Statusbar,
    current_file_label: Label,
    tab_counter: RefCell<u32>,
    self_weak: RefCell<std::rc::Weak<Self>>,
    temp_db: Rc<RefCell<TempDb>>,
    // Bottom toolbar buttons
    save_btn: RefCell<Option<Button>>,
    export_btn: RefCell<Option<Button>>,
    undo_btn: RefCell<Option<Button>>,
    redo_btn: RefCell<Option<Button>>,
}

impl MainWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Pak Assets Manager")
            .default_width(1400)
            .default_height(900)
            .build();

        // Header bar
        let header = HeaderBar::new();

        let scan_brave_btn = Button::builder()
            .label("Scan Brave")
            .build();

        header.pack_start(&scan_brave_btn);

        let current_file_label = Label::builder()
            .label("No file opened")
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(40)
            .width_chars(40)
            .build();
        header.set_title_widget(Some(&current_file_label));

        // Save button (initially disabled)
        let save_btn = Button::builder()
            .label("Save")
            .sensitive(false)
            .build();
        header.pack_end(&save_btn);

        // Export button (initially disabled)
        let export_btn = Button::builder()
            .label("Export to Brave")
            .sensitive(false)
            .build();
        header.pack_end(&export_btn);

        window.set_titlebar(Some(&header));

        // Main content
        let main_box = Box::new(Orientation::Vertical, 0);

        // Notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        main_box.append(&notebook);

        // Bottom toolbar
        let bottom_toolbar = Box::new(Orientation::Horizontal, 8);
        bottom_toolbar.set_margin_top(6);
        bottom_toolbar.set_margin_bottom(6);
        bottom_toolbar.set_margin_start(12);
        bottom_toolbar.set_margin_end(12);

        // Undo/Redo buttons
        let undo_btn = Button::from_icon_name("edit-undo-symbolic");
        undo_btn.set_tooltip_text(Some("Undo"));
        undo_btn.set_sensitive(false);

        let redo_btn = Button::from_icon_name("edit-redo-symbolic");
        redo_btn.set_tooltip_text(Some("Redo"));
        redo_btn.set_sensitive(false);

        bottom_toolbar.append(&undo_btn);
        bottom_toolbar.append(&redo_btn);
        bottom_toolbar.append(&Separator::new(Orientation::Vertical));

        // Tool size indicator
        let size_label = Label::new(Some("Tool Size: 8px"));
        bottom_toolbar.append(&size_label);

        bottom_toolbar.append(&Separator::new(Orientation::Vertical));

        // Status label
        let status_label = Label::new(Some("No changes"));
        status_label.set_hexpand(true);
        status_label.set_halign(gtk4::Align::Start);
        bottom_toolbar.append(&status_label);

        main_box.append(&bottom_toolbar);

        // Status bar
        let status_bar = Statusbar::new();
        status_bar.push(0, "Ready - Click 'Scan Brave' to browse pak files");
        main_box.append(&status_bar);

        window.set_child(Some(&main_box));

        let this = Rc::new(Self {
            window,
            tabs: RefCell::new(HashMap::new()),
            notebook,
            status_bar,
            current_file_label,
            tab_counter: RefCell::new(0),
            self_weak: RefCell::new(std::rc::Weak::new()),
            temp_db: Rc::new(RefCell::new(TempDb::new())),
            save_btn: RefCell::new(Some(save_btn)),
            export_btn: RefCell::new(Some(export_btn)),
            undo_btn: RefCell::new(Some(undo_btn)),
            redo_btn: RefCell::new(Some(redo_btn)),
        });

        // Set up self-referential weak pointer
        *this.self_weak.borrow_mut() = Rc::downgrade(&this);

        // Setup signals
        Self::setup_signals(&this, &scan_brave_btn);

        this
    }

    fn setup_signals(this: &Rc<Self>, scan_brave_btn: &Button) {
        println!("[DEBUG] Setting up signals...");

        // Scan Brave button
        let this_weak = this.self_weak.borrow().clone();
        scan_brave_btn.connect_clicked(move |_btn| {
            println!("[DEBUG] Scan Brave button clicked!");
            if let Some(this) = this_weak.upgrade() {
                println!("[DEBUG] Showing file browser");
                this.show_file_browser();
            }
        });

        // Notebook page switch signal
        let this_weak = Rc::downgrade(this);
        this.notebook.connect_switch_page(move |_notebook, _page, page_num| {
            if let Some(this) = this_weak.upgrade() {
                this.on_tab_switched(page_num);
            }
        });

        // Setup bottom toolbar button handlers
        Self::setup_toolbar_signals(this);
    }

    fn setup_toolbar_signals(this: &Rc<Self>) {
        // Undo button
        if let Some(ref undo_btn) = *this.undo_btn.borrow() {
            let this_weak = this.self_weak.borrow().clone();
            undo_btn.connect_clicked(move |_btn| {
                if let Some(this) = this_weak.upgrade() {
                    this.on_undo_clicked();
                }
            });
        }

        // Redo button
        if let Some(ref redo_btn) = *this.redo_btn.borrow() {
            let this_weak = this.self_weak.borrow().clone();
            redo_btn.connect_clicked(move |_btn| {
                if let Some(this) = this_weak.upgrade() {
                    this.on_redo_clicked();
                }
            });
        }

        // Save button
        if let Some(ref save_btn) = *this.save_btn.borrow() {
            let this_weak = this.self_weak.borrow().clone();
            save_btn.connect_clicked(move |_btn| {
                if let Some(this) = this_weak.upgrade() {
                    this.on_save_clicked();
                }
            });
        }

        // Export button
        if let Some(ref export_btn) = *this.export_btn.borrow() {
            let this_weak = this.self_weak.borrow().clone();
            export_btn.connect_clicked(move |_btn| {
                if let Some(this) = this_weak.upgrade() {
                    this.on_export_clicked();
                }
            });
        }
    }

    fn on_undo_clicked(&self) {
        let current_page = self.notebook.current_page();
        if let Some(page_num) = current_page {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(&page_num) {
                tab.editor_canvas.undo();
                self.update_toolbar_buttons(&tab);
            }
        }
    }

    fn on_redo_clicked(&self) {
        let current_page = self.notebook.current_page();
        if let Some(page_num) = current_page {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(&page_num) {
                tab.editor_canvas.redo();
                self.update_toolbar_buttons(&tab);
            }
        }
    }

    fn on_save_clicked(&self) {
        let current_page = self.notebook.current_page();
        if let Some(page_num) = current_page {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(&page_num) {
                self.save_current_resource(tab);
            }
        }
    }

    fn on_export_clicked(&self) {
        let current_page = self.notebook.current_page();
        if let Some(page_num) = current_page {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(&page_num) {
                self.export_to_brave(tab);
            }
        }
    }

    fn pak_key_for_tab(tab: &PakTab) -> String {
        tab.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn store_current_draft(&self, tab: &PakTab, action: &str) {
        if let Some(resource_id) = *tab.current_resource_id.borrow() {
            if let Some(image) = tab.editor_canvas.get_composite_image() {
                if let Ok(png_data) = encode_png(&image, 6) {
                    let pak_key = Self::pak_key_for_tab(tab);
                    self.temp_db
                        .borrow_mut()
                        .add_version(&pak_key, resource_id, action, png_data);
                }
            }
        }
    }

    fn maybe_store_active_resource_draft(&self, tab: &PakTab, resource_id: u16, action: &str) {
        if *tab.current_resource_id.borrow() == Some(resource_id) && tab.editor_canvas.has_changes() {
            self.store_current_draft(tab, action);
        }
    }

    fn get_resource_image_data(&self, tab: &PakTab, resource_id: u16) -> Option<Vec<u8>> {
        let pak_key = Self::pak_key_for_tab(tab);

        if let Some(version_id) = self.temp_db.borrow().get_current_version_id(&pak_key, resource_id) {
            if let Some(version) = self.temp_db.borrow().get_version(&pak_key, resource_id, &version_id) {
                return Some(version.image_data.clone());
            }
        }

        tab.pak_file
            .borrow()
            .resources
            .iter()
            .find(|r| r.id == resource_id)
            .map(|r| r.data.clone())
    }

    fn save_current_resource(&self, tab: &PakTab) -> bool {
        if let Some(resource_id) = *tab.current_resource_id.borrow() {
            if let Some(image) = tab.editor_canvas.get_composite_image() {
                match encode_png(&image, 6) {
                    Ok(png_data) => {
                        let pak_key = Self::pak_key_for_tab(tab);
                        // Update the pak file
                        let mut pak = tab.pak_file.borrow_mut();
                        if let Err(e) = pak.replace_resource(resource_id, png_data.clone()) {
                            self.show_error_dialog("Save Error", &format!("Failed to update resource: {}", e));
                            return false;
                        }

                        self.temp_db.borrow_mut().add_version(
                            &pak_key,
                            resource_id,
                            "Saved resource",
                            png_data,
                        );
                        self.temp_db.borrow_mut().mark_saved(&pak_key, resource_id);

                        // Clear changes flag
                        tab.editor_canvas.clear_changes();
                        self.update_toolbar_buttons(tab);
                        self.status_bar.push(0, &format!("Saved changes to resource {}", resource_id));
                        return true;
                    }
                    Err(e) => {
                        self.show_error_dialog("Export Error", &format!("Failed to encode PNG: {}", e));
                        return false;
                    }
                }
            }
        }

        false
    }

    fn export_to_brave(&self, tab: &PakTab) {
        if tab.editor_canvas.has_changes() && !self.save_current_resource(tab) {
            return;
        }

        // Then write the pak file back to the original path
        let pak = tab.pak_file.borrow();
        match pak.save(&tab.path) {
            Ok(_) => {
                self.status_bar.push(0, &format!("Exported to Brave: {}", tab.path.display()));
            }
            Err(e) => {
                self.show_error_dialog("Export Error", &format!("Failed to write pak file: {}", e));
            }
        }
    }

    fn update_toolbar_buttons(&self, tab: &PakTab) {
        let has_changes = tab.editor_canvas.has_changes();
        let can_undo = tab.editor_canvas.can_undo();
        let can_redo = tab.editor_canvas.can_redo();

        if let Some(ref save_btn) = *self.save_btn.borrow() {
            save_btn.set_sensitive(has_changes);
        }
        if let Some(ref export_btn) = *self.export_btn.borrow() {
            export_btn.set_sensitive(has_changes);
        }
        if let Some(ref undo_btn) = *self.undo_btn.borrow() {
            undo_btn.set_sensitive(can_undo);
        }
        if let Some(ref redo_btn) = *self.redo_btn.borrow() {
            redo_btn.set_sensitive(can_redo);
        }
    }

    fn show_file_browser(&self) {
        println!("[DEBUG] show_file_browser() called");
        self.status_bar.push(0, "Scanning for browsers...");

        let browsers = detect_browsers();
        println!("[DEBUG] Found {} browser(s)", browsers.len());

        if browsers.is_empty() {
            self.show_error_dialog("No browsers found", "Make sure Brave browser is installed.");
            self.status_bar.push(0, "No browsers found");
            return;
        }

        // Collect all pak files
        let mut all_pak_files: Vec<(String, PathBuf)> = Vec::new();
        for browser in &browsers {
            for pak_path in &browser.pak_files {
                let display_name = format!("{}", pak_path.display());
                all_pak_files.push((display_name, pak_path.clone()));
            }
        }

        println!("[DEBUG] Total pak files found: {}", all_pak_files.len());
        self.show_file_selection_dialog(&all_pak_files);
    }

    fn show_file_selection_dialog(&self, pak_files: &[(String, PathBuf)]) {
        let dialog = gtk4::Dialog::new();
        dialog.set_title(Some("Select Pak Files to Load"));
        dialog.set_transient_for(Some(&self.window));
        dialog.set_modal(true);
        dialog.set_default_size(600, 500);

        dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
        dialog.add_button("Load Selected", gtk4::ResponseType::Accept);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        let info_label = Label::new(Some(&format!(
            "Found {} pak files. Select which ones to open in tabs:",
            pak_files.len()
        )));
        info_label.set_halign(gtk4::Align::Start);
        content.append(&info_label);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

        let list_box = Box::new(Orientation::Vertical, 4);
        let checkboxes: RefCell<Vec<(CheckButton, PathBuf)>> = RefCell::new(Vec::new());

        for (display_name, path) in pak_files {
            let row = Box::new(Orientation::Horizontal, 8);
            row.set_margin_start(4);
            row.set_margin_end(4);

            let checkbox = CheckButton::new();
            // Select main pak files by default, skip locales to save memory
            let is_selected = !display_name.contains("/locales/");
            checkbox.set_active(is_selected);

            let label = Label::new(Some(display_name));
            label.set_halign(gtk4::Align::Start);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
            label.set_hexpand(true);

            row.append(&checkbox);
            row.append(&label);
            list_box.append(&row);

            checkboxes.borrow_mut().push((checkbox, path.clone()));
        }

        scrolled.set_child(Some(&list_box));
        content.append(&scrolled);

        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let select_all_btn = Button::with_label("Select All");
        let deselect_all_btn = Button::with_label("Deselect All");

        let checkboxes_clone = checkboxes.clone();
        select_all_btn.connect_clicked(move |_btn| {
            for (checkbox, _) in checkboxes_clone.borrow().iter() {
                checkbox.set_active(true);
            }
        });

        let checkboxes_clone2 = checkboxes.clone();
        deselect_all_btn.connect_clicked(move |_btn| {
            for (checkbox, _) in checkboxes_clone2.borrow().iter() {
                checkbox.set_active(false);
            }
        });

        button_box.append(&select_all_btn);
        button_box.append(&deselect_all_btn);
        content.append(&button_box);

        let this_weak = self.self_weak.borrow().clone();
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                let selected: Vec<PathBuf> = checkboxes
                    .borrow()
                    .iter()
                    .filter(|(cb, _)| cb.is_active())
                    .map(|(_, path)| path.clone())
                    .collect();

                println!("[DEBUG] User selected {} files to load", selected.len());

                if !selected.is_empty() {
                    if let Some(this) = this_weak.upgrade() {
                        // Load each selected file into a separate tab
                        for path in selected {
                            this.load_pak_file(&path);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.present();
        self.status_bar.push(0, &format!("Found {} pak files - select which to load", pak_files.len()));
    }

    fn load_pak_file(&self, path: &PathBuf) {
        println!("[DEBUG] Loading pak file: {:?}", path);
        self.status_bar.push(0, &format!("Loading {:?}...", path.file_name().unwrap_or_default()));

        match PakFile::load(path) {
            Ok(pak) => {
                println!("[DEBUG] Pak loaded: {} resources", pak.resources.len());
                self.create_tab(pak, path.clone());
            }
            Err(e) => {
                eprintln!("[DEBUG] Error loading pak: {}", e);
                self.show_error_dialog("Error Loading Pak File", &format!("{}", e));
                self.status_bar.push(0, &format!("Error loading {:?}", path.file_name().unwrap_or_default()));
            }
        }
    }

    fn create_tab(&self, pak: PakFile, path: PathBuf) {
        let tab_name = get_pak_display_name(&path);
        let resource_count = pak.resources.len();
        let image_count = pak.get_image_resources().len();

        println!("[DEBUG] Creating tab for {} ({} resources, {} images)", tab_name, resource_count, image_count);

        let resource_list = Rc::new(ResourceList::new());
        resource_list.set_resources(pak.get_image_resources().into_iter().cloned().collect());

        let editor_canvas = Rc::new(EditorCanvas::new());
        let tool_panel = Rc::new(ToolPanel::new());
        tool_panel.set_editor_canvas(Rc::clone(&editor_canvas));

        // Create a 3-pane layout: resource list | editor | tool panel
        let main_paned = Paned::new(Orientation::Horizontal);
        main_paned.set_wide_handle(true);
        main_paned.set_vexpand(true);
        main_paned.set_position(250);

        // Right side: editor + tool panel
        let right_paned = Paned::new(Orientation::Horizontal);
        right_paned.set_wide_handle(true);
        right_paned.set_position(800);
        right_paned.set_start_child(Some(editor_canvas.widget()));
        right_paned.set_end_child(Some(tool_panel.widget()));

        main_paned.set_start_child(Some(resource_list.widget()));
        main_paned.set_end_child(Some(&right_paned));

        let tab_box = Box::new(Orientation::Horizontal, 4);
        let label = Label::new(Some(&tab_name));
        tab_box.append(&label);

        let close_btn = Button::from_icon_name("window-close-symbolic");
        close_btn.set_has_frame(false);
        tab_box.append(&close_btn);

        let page_num = *self.tab_counter.borrow();
        *self.tab_counter.borrow_mut() += 1;

        let tab = PakTab {
            pak_file: RefCell::new(pak),
            path,
            resource_list: Rc::clone(&resource_list),
            editor_canvas: Rc::clone(&editor_canvas),
            tool_panel: Rc::clone(&tool_panel),
            current_resource_id: RefCell::new(None),
        };

        self.tabs.borrow_mut().insert(page_num, tab);
        self.notebook.append_page(&main_paned, Some(&tab_box));

        let total_pages = self.notebook.n_pages();
        if total_pages > 0 {
            self.notebook.set_current_page(Some(total_pages - 1));
        }

        let this_weak = self.self_weak.borrow().clone();
        close_btn.connect_clicked(move |_btn| {
            if let Some(this) = this_weak.upgrade() {
                this.close_tab(page_num);
            }
        });

        // Setup resource selection callback
        let editor_canvas_weak = Rc::downgrade(&editor_canvas);
        let this_weak = self.self_weak.borrow().clone();
        let page_num_for_cb = page_num;

        resource_list.on_selected(move |resource: &Resource| {
            println!("[DEBUG] Selected resource: {} ({} bytes, format: {:?})", resource.id, resource.data.len(), resource.format);
            if let Some(canvas) = editor_canvas_weak.upgrade() {
                if let Some(this) = this_weak.upgrade() {
                    let tabs = this.tabs.borrow();
                    if let Some(tab) = tabs.get(&page_num_for_cb) {
                        let previous_resource_id = *tab.current_resource_id.borrow();
                        if previous_resource_id != Some(resource.id) && tab.editor_canvas.has_changes() {
                            this.store_current_draft(tab, "Switched resource");
                        }

                        if let Some(data) = this.get_resource_image_data(tab, resource.id) {
                            println!("[DEBUG] Attempting to load image from {} bytes", data.len());

                            match image::load_from_memory(&data) {
                                Ok(img) => {
                                    let (w, h) = (img.width(), img.height());
                                    println!("[DEBUG] Image loaded successfully: {}x{}", w, h);
                                    canvas.set_base_image(&img);
                                    println!("[DEBUG] Image set to canvas");

                                    let this_weak2 = this_weak.clone();
                                    canvas.set_on_changed_callback(move |has_changes| {
                                        println!("[DEBUG] Changes state: {}", has_changes);
                                        if let Some(this) = this_weak2.upgrade() {
                                            let current_page = this.notebook.current_page();
                                            if let Some(page_num) = current_page {
                                                let tabs = this.tabs.borrow();
                                                if let Some(tab) = tabs.get(&page_num) {
                                                    this.update_toolbar_buttons(tab);
                                                }
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    eprintln!("[DEBUG] Failed to load image: {}", e);
                                }
                            }
                        }

                        *tab.current_resource_id.borrow_mut() = Some(resource.id);
                    }
                }
            }
        });

        // Setup right-click context menu
        let this_weak = self.self_weak.borrow().clone();
        let page_num_for_menu = page_num;
        resource_list.on_context_menu(move |resource: &Resource, x, y| {
            println!("[DEBUG] Right-click on resource {} at ({}, {})", resource.id, x, y);
            if let Some(this) = this_weak.upgrade() {
                this.show_resource_context_menu(resource, x, y, page_num_for_menu);
            }
        });
        
        // Setup three-dot menu: Open Externally
        let this_weak = self.self_weak.borrow().clone();
        let page_num_for_external = page_num;
        resource_list.on_open_external(move |resource: &Resource| {
            println!("[DEBUG] Open externally: resource {}", resource.id);
            if let Some(this) = this_weak.upgrade() {
                this.open_resource_externally(resource, page_num_for_external);
            }
        });
        
        // Setup three-dot menu: Version History
        let this_weak = self.self_weak.borrow().clone();
        let page_num_for_history = page_num;
        resource_list.on_version_history(move |resource: &Resource| {
            println!("[DEBUG] Version history: resource {}", resource.id);
            if let Some(this) = this_weak.upgrade() {
                this.show_version_history(resource, page_num_for_history);
            }
        });

        self.status_bar.push(0, &format!(
            "Loaded '{}' - {} resources ({} images)",
            tab_name, resource_count, image_count
        ));
    }

    fn close_tab(&self, page_num: u32) {
        if let Some(page_widget) = self.notebook.nth_page(Some(page_num)) {
            if let Some(actual_idx) = self.notebook.page_num(&page_widget) {
                self.notebook.remove_page(Some(actual_idx));
            }
            self.tabs.borrow_mut().remove(&page_num);

            if self.tabs.borrow().is_empty() {
                self.current_file_label.set_text("No file opened");
                self.status_bar.push(0, "All tabs closed");
                // Disable buttons
                if let Some(ref save_btn) = *self.save_btn.borrow() {
                    save_btn.set_sensitive(false);
                }
                if let Some(ref export_btn) = *self.export_btn.borrow() {
                    export_btn.set_sensitive(false);
                }
            }
        }
    }

    fn on_tab_switched(&self, page_num: u32) {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.get(&page_num) {
            let name = get_pak_display_name(&tab.path);
            self.current_file_label.set_text(&name);
            self.status_bar.push(0, &format!("Switched to tab: {}", name));
            self.update_toolbar_buttons(tab);
        }
    }

    fn show_resource_context_menu(&self, resource: &Resource, _x: f64, _y: f64, page_num: u32) {
        // Create a simple menu using a popover
        let menu_box = Box::new(Orientation::Vertical, 0);
        menu_box.add_css_class("menu");
        
        // Open externally button
        let open_btn = Button::builder()
            .label("Open with External Application")
            .has_frame(false)
            .build();
        
        // Save to file button
        let save_btn = Button::builder()
            .label("Save to File...")
            .has_frame(false)
            .build();
        
        menu_box.append(&open_btn);
        menu_box.append(&gtk4::Separator::new(Orientation::Horizontal));
        menu_box.append(&save_btn);
        
        // Create popover menu
        let popover = gtk4::Popover::builder()
            .child(&menu_box)
            .autohide(true)
            .build();
        
        // Position the popover near the resource list
        popover.set_parent(&self.notebook);

        let resource_id = resource.id;
        let resource_data = {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(&page_num) {
                self.maybe_store_active_resource_draft(tab, resource_id, "Opened resource menu");
                self.get_resource_image_data(tab, resource_id)
                    .unwrap_or_else(|| resource.data.clone())
            } else {
                resource.data.clone()
            }
        };
        let resource_data_open = resource_data.clone();
        let resource_data_save = resource_data.clone();
        let window_weak = self.window.downgrade();
        
        open_btn.connect_clicked(move |_btn| {
            // Save to temp file and open with external app
            if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                let temp_file = temp_dir.join(format!("pak_resource_{}.png", resource_id));
                if let Err(e) = std::fs::write(&temp_file, &resource_data_open) {
                    eprintln!("[DEBUG] Failed to write temp file: {}", e);
                    return;
                }
                
                // Open with xdg-open (Linux) or equivalent
                let result = std::process::Command::new("xdg-open")
                    .arg(&temp_file)
                    .spawn();
                
                if let Err(e) = result {
                    eprintln!("[DEBUG] Failed to open external app: {}", e);
                    // Try gnome-specific apps
                    let _ = std::process::Command::new("eog")  // Eye of GNOME
                        .arg(&temp_file)
                        .spawn();
                }
            }
            
            if let Some(window) = window_weak.upgrade() {
                let popover = window.first_child()
                    .and_then(|c| c.first_child())
                    .and_then(|c| c.downcast::<gtk4::Popover>().ok());
                if let Some(p) = popover {
                    p.popdown();
                }
            }
        });
        
        let resource_data2 = resource_data_save.clone();
        let resource_id2 = resource.id;
        
        save_btn.connect_clicked(move |btn| {
            // Use native file dialog through zenity or similar
            let initial_name = format!("resource_{}.png", resource_id2);
            let data_clone = resource_data2.clone();
            
            // Try to use zenity for file chooser
            let result = std::process::Command::new("zenity")
                .args(&["--file-selection", "--save", "--filename", &initial_name, "--title", "Save Image"])
                .output();
            
            let path = match result {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                }
                _ => {
                    // Fallback to temp file
                    let temp_dir = std::env::temp_dir();
                    let temp_file = temp_dir.join(&initial_name);
                    temp_file.to_string_lossy().to_string()
                }
            };
            
            if !path.is_empty() {
                if let Err(e) = std::fs::write(&path, &data_clone) {
                    eprintln!("[DEBUG] Failed to save file: {}", e);
                } else {
                    println!("[DEBUG] Saved to {}", path);
                }
            }
            
            // Close the popover
            if let Some(parent) = btn.parent() {
                if let Some(popover) = parent.parent().and_then(|p| p.downcast::<gtk4::Popover>().ok()) {
                    popover.popdown();
                }
            }
        });
        
        // Show the popover - position at mouse coordinates relative to window
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(_x as i32, _y as i32, 1, 1)));
        popover.popup();
    }
    
    fn open_resource_externally(&self, resource: &Resource, page_num: u32) {
        // Save to temp file and open with external app
        if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
            let data = {
                let tabs = self.tabs.borrow();
                if let Some(tab) = tabs.get(&page_num) {
                    self.maybe_store_active_resource_draft(tab, resource.id, "Opened externally");
                    self.get_resource_image_data(tab, resource.id)
                        .unwrap_or_else(|| resource.data.clone())
                } else {
                    resource.data.clone()
                }
            };

            let ext = match resource.format {
                Some(crate::pak::ImageFormat::Png) => "png",
                Some(crate::pak::ImageFormat::Webp) => "webp",
                _ => "bin",
            };
            let temp_file = temp_dir.join(format!("pak_resource_{}.{}", resource.id, ext));
            if let Err(e) = std::fs::write(&temp_file, &data) {
                eprintln!("[DEBUG] Failed to write temp file: {}", e);
                self.show_error_dialog("Error", &format!("Failed to write temp file: {}", e));
                return;
            }
            
            // Open with xdg-open (Linux) or equivalent
            let result = std::process::Command::new("xdg-open")
                .arg(&temp_file)
                .spawn();
            
            if let Err(e) = result {
                eprintln!("[DEBUG] Failed to open external app: {}", e);
                // Try gnome-specific apps
                let _ = std::process::Command::new("eog")  // Eye of GNOME
                    .arg(&temp_file)
                    .spawn();
            }
        }
    }
    
    fn show_version_history(&self, resource: &Resource, page_num: u32) {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.get(&page_num) {
            self.maybe_store_active_resource_draft(tab, resource.id, "Opened version history");
            let pak_name = tab.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            // Create version history dialog
            let dialog = VersionHistoryDialog::new(
                &self.window,
                resource.id,
                pak_name,
                self.temp_db.clone(),
            );
            
            // Set up restore callback
            let canvas = tab.editor_canvas.clone();
            let temp_db = self.temp_db.clone();
            let pak_name = pak_name.to_string();
            let resource_id = resource.id;
            dialog.set_on_restore(move |image_data: &[u8]| {
                // Load the restored image into the canvas
                if let Ok(img) = image::load_from_memory(image_data) {
                    canvas.set_base_image(&img);
                    temp_db.borrow_mut().add_version(
                        &pak_name,
                        resource_id,
                        "Restored version",
                        image_data.to_vec(),
                    );
                    canvas.mark_changed();
                }
            });
            
            dialog.show();
        }
    }

    fn show_error_dialog(&self, title: &str, message: &str) {
        let dialog = gtk4::MessageDialog::new(
            Some(&self.window),
            gtk4::DialogFlags::MODAL,
            gtk4::MessageType::Error,
            gtk4::ButtonsType::Ok,
            title,
        );
        dialog.set_secondary_text(Some(message));
        dialog.connect_response(|d, _| d.close());
        dialog.present();
    }

    pub fn present(&self) {
        self.window.present();
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        panic!("MainWindow requires an Application");
    }
}
