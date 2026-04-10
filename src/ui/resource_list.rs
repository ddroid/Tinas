use gtk4::prelude::*;
use gtk4::{Label, ListBox, ListBoxRow, ScrolledWindow, MenuButton, Popover, gio};
use std::cell::RefCell;
use std::rc::Rc;

use crate::pak::Resource;

type SelectedCallback = std::boxed::Box<dyn Fn(&Resource)>;
type ContextMenuCallback = std::boxed::Box<dyn Fn(&Resource, f64, f64)>;
type OpenExternalCallback = std::boxed::Box<dyn Fn(&Resource)>;
type VersionHistoryCallback = std::boxed::Box<dyn Fn(&Resource)>;

pub struct ResourceList {
    container: ScrolledWindow,
    list_box: ListBox,
    resources: Rc<RefCell<Vec<Resource>>>,
    on_selected: Rc<RefCell<Option<SelectedCallback>>>,
    on_context_menu: Rc<RefCell<Option<ContextMenuCallback>>>,
    on_open_external: Rc<RefCell<Option<OpenExternalCallback>>>,
    on_version_history: Rc<RefCell<Option<VersionHistoryCallback>>>,
}

impl ResourceList {
    pub fn new() -> Self {
        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        
        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&list_box)
            .width_request(200)
            .build();
        
        let this = Self {
            container,
            list_box,
            resources: Rc::new(RefCell::new(Vec::new())),
            on_selected: Rc::new(RefCell::new(None)),
            on_context_menu: Rc::new(RefCell::new(None)),
            on_open_external: Rc::new(RefCell::new(None)),
            on_version_history: Rc::new(RefCell::new(None)),
        };
        
        this.setup_signals();
        this.setup_right_click();
        this
    }
    
    pub fn on_open_external<F>(&self, callback: F)
    where
        F: Fn(&Resource) + 'static,
    {
        *self.on_open_external.borrow_mut() = Some(Box::new(callback));
    }
    
    pub fn on_version_history<F>(&self, callback: F)
    where
        F: Fn(&Resource) + 'static,
    {
        *self.on_version_history.borrow_mut() = Some(Box::new(callback));
    }
    
    fn get_row_index(&self, target_row: &ListBoxRow) -> Option<usize> {
        // Iterate through children to find index
        let mut index = 0;
        let mut child = self.list_box.first_child();
        while let Some(c) = child {
            if let Some(row) = c.downcast_ref::<ListBoxRow>() {
                if row == target_row {
                    return Some(index);
                }
                index += 1;
            }
            child = c.next_sibling();
        }
        None
    }
    
    fn setup_signals(&self) {
        let on_selected = Rc::clone(&self.on_selected);
        let resources = self.resources.clone();
        let list_box_weak = self.list_box.downgrade();
        
        self.list_box.connect_selected_rows_changed(move |_list_box| {
            println!("[RESOURCE LIST] Selection changed!");
            if let Some(list_box) = list_box_weak.upgrade() {
                if let Some(row) = list_box.selected_row() {
                    let index = row.index() as usize;
                    println!("[RESOURCE LIST] Row selected, index: {}", index);
                    
                    if let Some(ref callback) = *on_selected.borrow() {
                        if let Some(resource) = resources.borrow().get(index) {
                            println!("[RESOURCE LIST] Calling callback for resource {}", resource.id);
                            callback(resource);
                        } else {
                            println!("[RESOURCE LIST] No resource at index {} (total resources: {})", index, resources.borrow().len());
                        }
                    } else {
                        println!("[RESOURCE LIST] No callback set!");
                    }
                } else {
                    println!("[RESOURCE LIST] No row selected");
                }
            }
        });
    }
    
    fn setup_right_click(&self) {
        let resources = self.resources.clone();
        let on_context_menu = self.on_context_menu.clone();
        let list_box_weak = self.list_box.downgrade();
        
        // Create right-click gesture (button 3 = right click)
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
        
        gesture.connect_pressed(move |gesture, _n_press, x, y| {
            // Get the list_box from weak reference
            let list_box = match list_box_weak.upgrade() {
                Some(lb) => lb,
                None => return,
            };
            
            // Use pick to find the widget at coordinates
            let picked = list_box.pick(x as f64, y as f64, gtk4::PickFlags::DEFAULT);
            
            if let Some(widget) = picked {
                // Walk up to find the ListBoxRow
                let mut current: Option<gtk4::Widget> = Some(widget);
                while let Some(w) = current {
                    if let Some(row) = w.downcast_ref::<ListBoxRow>() {
                        // Found the row, get index by position
                        let mut index = 0;
                        let mut child = list_box.first_child();
                        while let Some(c) = child {
                            if let Some(child_row) = c.downcast_ref::<ListBoxRow>() {
                                if child_row == row {
                                    if let Some(resource) = resources.borrow().get(index) {
                                        if let Some(ref callback) = *on_context_menu.borrow() {
                                            callback(resource, x, y);
                                        }
                                    }
                                    break;
                                }
                                index += 1;
                            }
                            child = c.next_sibling();
                        }
                        break;
                    }
                    current = w.parent();
                }
            }
        });
        
        self.list_box.add_controller(gesture);
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }
    
    pub fn set_resources(&self, resources: Vec<Resource>) {
        println!("[RESOURCE LIST] set_resources called with {} resources", resources.len());
        self.clear();
        
        *self.resources.borrow_mut() = resources;
        println!("[RESOURCE LIST] Resources stored, total: {}", self.resources.borrow().len());
        
        for (index, resource) in self.resources.borrow().iter().enumerate() {
            let row = self.create_resource_row(resource, index);
            self.list_box.append(&row);
        }
        println!("[RESOURCE LIST] {} rows added to list", self.list_box.observe_children().n_items());
    }
    
    fn create_resource_row(&self, resource: &Resource, index: usize) -> ListBoxRow {
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        hbox.set_hexpand(true);
        
        // Left side: Resource info
        let info_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        info_box.set_hexpand(true);
        info_box.set_halign(gtk4::Align::Start);
        
        // Resource ID label
        let id_label = Label::new(Some(&format!("ID: {}", resource.id)));
        info_box.append(&id_label);
        
        // Size info if available
        if let (Some(w), Some(h)) = (resource.width, resource.height) {
            let size_label = Label::new(Some(&format!("({}x{})", w, h)));
            size_label.add_css_class("dim-label");
            info_box.append(&size_label);
        }
        
        // Format label
        let format_str = match resource.format {
            Some(crate::pak::ImageFormat::Png) => "PNG",
            Some(crate::pak::ImageFormat::Webp) => "WebP",
            _ => "Unknown",
        };
        let format_label = Label::new(Some(format_str));
        format_label.add_css_class("dim-label");
        info_box.append(&format_label);
        
        hbox.append(&info_box);
        
        // Right side: Three-dot menu button
        let menu_button = MenuButton::new();
        menu_button.set_icon_name("view-more-symbolic");
        menu_button.set_valign(gtk4::Align::Center);
        menu_button.add_css_class("flat");
        menu_button.add_css_class("circular");
        
        // Create menu model
        let menu = gio::Menu::new();
        menu.append(Some("Open Externally"), Some(&format!("resource.open_external.{}", index)));
        menu.append(Some("Version History"), Some(&format!("resource.version_history.{}", index)));
        
        menu_button.set_menu_model(Some(&menu));
        
        // Use a popover with custom buttons
        let popover = Popover::new();
        let popover_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        popover_box.set_margin_top(8);
        popover_box.set_margin_bottom(8);
        popover_box.set_margin_start(8);
        popover_box.set_margin_end(8);
        
        // Open Externally button
        let open_btn = gtk4::Button::with_label("Open Externally");
        open_btn.add_css_class("flat");
        let on_open_external = self.on_open_external.clone();
        let resources = self.resources.clone();
        let resource_id = resource.id;
        open_btn.connect_clicked(move |_| {
            if let Some(ref callback) = *on_open_external.borrow() {
                if let Some(res) = resources.borrow().iter().find(|r| r.id == resource_id) {
                    callback(res);
                }
            }
        });
        popover_box.append(&open_btn);
        
        // Version History button
        let history_btn = gtk4::Button::with_label("Version History");
        history_btn.add_css_class("flat");
        let on_version_history = self.on_version_history.clone();
        let resources = self.resources.clone();
        let resource_id = resource.id;
        history_btn.connect_clicked(move |_| {
            if let Some(ref callback) = *on_version_history.borrow() {
                if let Some(res) = resources.borrow().iter().find(|r| r.id == resource_id) {
                    callback(res);
                }
            }
        });
        popover_box.append(&history_btn);
        
        popover.set_child(Some(&popover_box));
        popover.set_autohide(true);
        popover.set_has_arrow(true);
        popover.set_position(gtk4::PositionType::Right);
        
        menu_button.set_popover(Some(&popover));
        menu_button.set_create_popup_func(|_| {}); // Disable default popup
        
        hbox.append(&menu_button);
        
        let row = ListBoxRow::new();
        row.set_child(Some(&hbox));
        row
    }
    
    pub fn clear(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.resources.borrow_mut().clear();
    }
    
    pub fn on_selected<F>(&self, callback: F)
    where
        F: Fn(&Resource) + 'static,
    {
        *self.on_selected.borrow_mut() = Some(Box::new(callback));
    }
    
    pub fn on_context_menu<F>(&self, callback: F)
    where
        F: Fn(&Resource, f64, f64) + 'static,
    {
        *self.on_context_menu.borrow_mut() = Some(Box::new(callback));
    }
}

impl Default for ResourceList {
    fn default() -> Self {
        Self::new()
    }
}
