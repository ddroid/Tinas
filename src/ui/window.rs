use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, HeaderBar, Box, Button,
           Orientation, Paned, Label, Statusbar, Notebook, ScrolledWindow, CheckButton};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::pak::PakFile;
use crate::pak::Resource;
use crate::ui::resource_list::ResourceList;
use crate::ui::editor_canvas::EditorCanvas;
use crate::browser_detector::{detect_browsers, get_pak_display_name};

/// Data for a single pak file tab
struct PakTab {
    pak_file: PakFile,
    path: PathBuf,
    resource_list: Rc<ResourceList>,
    editor_canvas: Rc<EditorCanvas>,
}

pub struct MainWindow {
    window: ApplicationWindow,
    tabs: RefCell<HashMap<u32, PakTab>>, // notebook page num -> PakTab
    notebook: Notebook,
    status_bar: Statusbar,
    current_file_label: Label,
    tab_counter: RefCell<u32>,
    self_weak: RefCell<std::rc::Weak<Self>>,
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

        let current_file_label = Label::new(Some("No file opened"));
        header.set_title_widget(Some(&current_file_label));

        window.set_titlebar(Some(&header));

        // Main content
        let main_box = Box::new(Orientation::Vertical, 0);

        // Notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        main_box.append(&notebook);

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

        let paned = Paned::new(Orientation::Horizontal);
        paned.set_wide_handle(true);
        paned.set_vexpand(true);
        paned.set_start_child(Some(resource_list.widget()));
        paned.set_end_child(Some(editor_canvas.widget()));
        paned.set_position(300);

        let tab_box = Box::new(Orientation::Horizontal, 4);
        let label = Label::new(Some(&tab_name));
        tab_box.append(&label);

        let close_btn = Button::from_icon_name("window-close-symbolic");
        close_btn.set_has_frame(false);
        tab_box.append(&close_btn);

        let page_num = *self.tab_counter.borrow();
        *self.tab_counter.borrow_mut() += 1;

        let tab = PakTab {
            pak_file: pak,
            path,
            resource_list: Rc::clone(&resource_list),
            editor_canvas: Rc::clone(&editor_canvas),
        };

        self.tabs.borrow_mut().insert(page_num, tab);
        self.notebook.append_page(&paned, Some(&tab_box));
        
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

        let editor_canvas_weak = Rc::downgrade(&editor_canvas);
        resource_list.on_selected(move |resource: &Resource| {
            println!("[DEBUG] Selected resource: {} ({} bytes)", resource.id, resource.data.len());
            if let Some(canvas) = editor_canvas_weak.upgrade() {
                canvas.set_base_image(&image::DynamicImage::new_rgba8(100, 100));
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
            }
        }
    }

    fn on_tab_switched(&self, page_num: u32) {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.get(&page_num) {
            let name = get_pak_display_name(&tab.path);
            self.current_file_label.set_text(&name);
            self.status_bar.push(0, &format!("Switched to tab: {}", name));
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
