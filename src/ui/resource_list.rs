use gtk4::prelude::*;
use gtk4::{Label, ListBox, ListBoxRow, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

use crate::pak::Resource;

type SelectedCallback = std::boxed::Box<dyn Fn(&Resource)>;

pub struct ResourceList {
    container: ScrolledWindow,
    list_box: ListBox,
    resources: RefCell<Vec<Resource>>,
    on_selected: Rc<RefCell<Option<SelectedCallback>>>,
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
            resources: RefCell::new(Vec::new()),
            on_selected: Rc::new(RefCell::new(None)),
        };
        
        this.setup_signals();
        this
    }
    
    fn setup_signals(&self) {
        let on_selected = Rc::clone(&self.on_selected);
        let resources = self.resources.clone();
        
        self.list_box.connect_selected_rows_changed(move |list_box| {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                if let Some(ref callback) = *on_selected.borrow() {
                    if let Some(resource) = resources.borrow().get(index) {
                        callback(resource);
                    }
                }
            }
        });
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }
    
    pub fn set_resources(&self, resources: Vec<Resource>) {
        self.clear();
        
        *self.resources.borrow_mut() = resources;
        
        for resource in self.resources.borrow().iter() {
            let row = self.create_resource_row(resource);
            self.list_box.append(&row);
        }
    }
    
    fn create_resource_row(&self, resource: &Resource) -> ListBoxRow {
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        
        // Resource ID label
        let id_label = Label::new(Some(&format!("ID: {}", resource.id)));
        hbox.append(&id_label);
        
        // Size info if available
        if let (Some(w), Some(h)) = (resource.width, resource.height) {
            let size_label = Label::new(Some(&format!("({}x{})", w, h)));
            size_label.add_css_class("dim-label");
            hbox.append(&size_label);
        }
        
        // Format label
        let format_str = match resource.format {
            Some(crate::pak::ImageFormat::Png) => "PNG",
            Some(crate::pak::ImageFormat::Webp) => "WebP",
            _ => "Unknown",
        };
        let format_label = Label::new(Some(format_str));
        format_label.add_css_class("dim-label");
        hbox.append(&format_label);
        
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
}

impl Default for ResourceList {
    fn default() -> Self {
        Self::new()
    }
}
