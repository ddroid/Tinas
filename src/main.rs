mod pak;
mod image_processor;
mod editor;
mod ui;
mod browser_detector;

use gtk4::prelude::*;
use gtk4::Application;
use ui::window::MainWindow;
use std::cell::RefCell;

const APP_ID: &str = "com.example.PakAssetsManager";

// Store window in a static to keep it alive (GTK is single-threaded)
thread_local! {
    static MAIN_WINDOW: RefCell<Option<std::rc::Rc<MainWindow>>> = RefCell::new(None);
}

fn main() -> glib::ExitCode {
    // Initialize GTK
    let app = Application::builder()
        .application_id(APP_ID)
        .build();
    
    app.connect_activate(build_ui);
    
    app.run()
}

fn build_ui(app: &Application) {
    let window = MainWindow::new(app);

    // Store window in thread_local to keep it alive
    // This prevents the Rc from being dropped when build_ui returns
    MAIN_WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));

    // Note: Auto-load disabled - user must click "Scan Brave" button manually
    // This helps with debugging button click issues
    // glib::idle_add_local_once(glib::clone!(@strong window => move || {
    //     window.auto_load_brave_paks();
    // }));

    window.present();
}
