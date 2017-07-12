//! # Basic Sample
//!
//! This sample demonstrates how to create a toplevel `window`, set its title, size and position, how to add a `button` to this `window` and how to connect signals with actions.

extern crate gtk;

use gtk::prelude::*;
use gtk::{Button, Window, Value, WindowType, FileChooserDialog, TreeView,
         TreeStore, CellRendererText, CellRendererToggle, TreeViewColumn, TreePath};

extern crate svd_parser as svd;
use svd::Register;


use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};

const FILE: &str = "registers.txt";


    
fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    
    let window = Window::new(WindowType::Toplevel);
    let view = TreeView::new();
    //let open_button = Button::new_with_label("Open");
    let ok_button = Button::new_with_label("Ok");
    let apply_button = Button::new_with_label("Apply");
    let cancel_button = Button::new_with_label("Cancel");
    
    let mut svd_filename:     Option<String> = None;
    let mut store:        Option<TreeStore> = None;
    
    window.set_title("SVD");
    window.set_border_width(10);
    
    let cell_name = CellRendererText::new();
    let column_name = TreeViewColumn::new();
    column_name.pack_start(&cell_name, true);
    column_name.add_attribute(&cell_name, "text", 0);
    column_name.set_title("Name");
    view.append_column(&column_name);
    
    let cell_in_out = CellRendererToggle::new();
    let column_in_out = TreeViewColumn::new();
    column_in_out.pack_start(&cell_in_out, true);
    column_in_out.add_attribute(&cell_in_out, "active", 1);
    column_in_out.set_title("Out?");
    view.append_column(&column_in_out);
    
    let cell_address = CellRendererText::new();
    let column_address = TreeViewColumn::new();
    column_address.pack_start(&cell_address, true);
    column_address.add_attribute(&cell_address, "text", 2);
    column_address.set_title("Address");
    view.append_column(&column_address);
    
    let cell_description = CellRendererText::new();
    let column_description = TreeViewColumn::new();
    column_description.pack_start(&cell_description, true);
    column_description.add_attribute(&cell_description, "text", 3);
    column_description.set_title("Description");
    view.append_column(&column_description);
    
    let scrolled_window = gtk::ScrolledWindow::new(None, None);
    scrolled_window.set_policy(
        gtk::PolicyType::Always, gtk::PolicyType::Always);
    scrolled_window.add_with_viewport(&view);
    
    scrolled_window.set_size_request(500, 600);
    
    let grid = gtk::Grid::new();
    grid.set_row_spacing(5);
    

    //grid.attach(&open_button,     0, 0, 1, 1);
    grid.attach(&scrolled_window,   0, 1, 5, 1);
    grid.attach(&ok_button,       2, 2, 1, 1);
    grid.attach(&apply_button,    3, 2, 1, 1);
    grid.attach(&cancel_button,   4, 2, 1, 1);
    
    window.add(&grid);
         
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });  
    window.show_all();
    
    let mut fflag = false;
    let text = &mut String::new();
    if let Ok(mut f) = File::open(FILE) {
        f.read_to_string(text).expect("Unable to read file");
        let mut lines = text.lines().map(|l| l.trim());
        if let Some(filename) = lines.next() {
            println!("SVD File {}", filename);
            let mut regs = Vec::new();
            for l in lines {
                regs.push(l.split_whitespace().collect::<Vec<_>>()[0]);
            }
            println!("{:?}", regs);
            store = load_svd(Path::new(&filename), regs);
            svd_filename = Some(filename.to_string());
            if let Some(ref st) = store {
                fflag = true;
                view.set_model(st);
            }
        }
    }
    if !fflag {
        if let Some(pathbuf) = open_file(&window) {
            println!("Open SVD File {:?}", pathbuf); 
            store = load_svd(pathbuf.as_path(), vec![]);
            if let Some(ref st) = store {
                svd_filename = pathbuf.into_os_string().into_string().ok();
                view.set_model(st);
            }
        }
    }
    {
        let store = store.clone();
        cell_in_out.connect_toggled(move |_,path| {
            if let Some(ref st) = store {
                on_toggle(st, &path)
            }
        });
    }
    {
        let store = store.clone();
        let svd_filename = svd_filename.clone();
        ok_button.connect_clicked(move |_| {
            if let Some(ref st) = store {
                if let Some(ref svd_file) = svd_filename {
                    save_data(st, svd_file).expect("Unable to save file");
                }
            }
            gtk::main_quit();
        });
    }
    {
        let store = store.clone();
        let svd_filename = svd_filename.clone();
        apply_button.connect_clicked(move |_| {
            if let Some(ref st) = store {
                if let Some(ref svd_file) = svd_filename {
                    save_data(st, svd_file).expect("Unable to save file");
                }
            }
        });
    }
    cancel_button.connect_clicked(|_| { gtk::main_quit();  });
    /*
    {
        let window = window.clone();
        open_button.connect_clicked(move |_| {
            if let Some(pathbuf) = open_file(&window) {
            println!("Open SVD File {:?}", pathbuf); 
            store = load_svd(pathbuf.as_path(), vec![]);
            if let Some(ref st) = store {
                svd_filename = pathbuf.into_os_string().into_string().ok();
                view.set_model(st);
            }
        }
            
        });
    }*/
    gtk::main();
}

fn open_file(window: &Window) -> Option<PathBuf> {
    let dialog = FileChooserDialog::new(Some("Please choose a file"), Some(window),
        gtk::FileChooserAction::Open);
    dialog.add_button("Cancel", 0);//gtk::ResponseType::Cancel);
    dialog.add_button("Open", 1);//gtk::ResponseType::Ok);
    let response = dialog.run();
    let pathbuf = match response {
        1 => dialog.get_filename(),
        _ =>  None
    };
    dialog.destroy();
    pathbuf
}

fn load_svd (svd_path: &Path, regs: Vec<&str>) -> Option<TreeStore> {
    let xml = &mut String::new();
    File::open(&svd_path).unwrap().read_to_string(xml).expect("Unable to read file");
    let device = svd::parse(xml);
    let periphs = device.peripherals;
    
    let store = TreeStore::new(&[String::static_type(), gtk::Type::Bool, String::static_type(), String::static_type()]);
    for p in &periphs {
        let paddr = p.base_address;
        let piter = store.append(None);
        let pdesc = match p.description {
            Some(ref s) => s.replace('\n', " "),
            None => "".to_string()
        };
        let pbase = match p.derived_from { // need correct
            Some(ref s) => {
                let mut pb = p;
                let mut k = 1000; 
                for i in 0..periphs.len() {
                    if *s == periphs[i].name {
                        k = i;
                        break;
                    }
                }
                if k != 1000 {
                    pb = &periphs[k];
                }
                pb
            },
            None => p
        };
        store.set(&piter, &[0, 1, 2, 3], &[&p.name, &false, &format!("0x{:08x}", paddr), &pdesc]);
        if let Some(ref rs) = pbase.registers {
            for reg in rs {
                match reg {
                    &Register::Single(ref r) | &Register::Array(ref r, _) => {
                        let raddr = paddr + r.address_offset;
                        let rname = format!("{}.{}", p.name, r.name);
                        let rdesc = r.description.replace('\n', " ");
                        
                        let mut enabled = false;
                        for rn in &regs {
                            if *rn == rname {
                                enabled = true;
                                break;
                            }
                        }
                        if enabled { println!("{}", rname); }
                        let riter = store.append(&piter);
                        store.set(&riter, &[0, 1, 2, 3], &[&r.name, &enabled, &format!("0x{:08x}", raddr), &rdesc]);
                    }
                }
                    
            }
        }
        set_piter_selected(&store, &piter);
    }
    Some(store)
}

fn save_data (store: &TreeStore, svd_file: &String) -> Result<(), std::io::Error> {
    let mut s = svd_file.to_owned() + "\n";
    if let Some(ref piter) = store.get_iter_first() {
        loop {
            if let Some(ref citer) = store.iter_children(piter) {
                loop {
                    if store.get_value(citer, 1).get::<bool>().unwrap() {
                        s += &format!("{}.{} {}\n", store.get_value(&piter, 0).get::<String>().unwrap(),
                                                    store.get_value(&citer, 0).get::<String>().unwrap(),
                                                    store.get_value(&citer, 2).get::<String>().unwrap());
                    }
                    if !store.iter_next(citer) { break; }
                }
            }
            if !store.iter_next(piter)  { break; }
        }
    }
    println!("{}", s);
    let mut f = File::create(FILE)?;
    f.write_fmt(format_args!("{}", s))?;
    f.flush()?;
    Ok(())
}

fn on_toggle(st: &TreeStore, path: &TreePath) {
    if let Some(iter) = st.get_iter(path) {
        let current_value = !st.get_value(&iter, 1).get::<bool>().unwrap();
        st.set_value(&iter, 1, &Value::from(&current_value));
        
        if path.get_depth() == 1 {
            let piter = iter;
            if let Some(ref citer) = st.iter_children(&piter) {
                loop {
                    st.set_value(citer, 1, &Value::from(&current_value));
                    if !st.iter_next(citer) { break; }
                }
            }
            println!("{} {}", st.get_value(&piter, 0).get::<String>().unwrap(),
                              if current_value == true {"enabled"} else {"disabled"});
        }
        else {
            let citer = iter;
            if let Some(ref piter) = st.iter_parent(&citer) {
                let all_selected = set_piter_selected (st, &piter);
                if all_selected {
                    println!("{} {}", st.get_value(&piter, 0).get::<String>().unwrap(),
                                      if current_value == true {"enabled"} else {"disabled"});
                }
                else {
                    println!("{}.{} {}", st.get_value(&piter, 0).get::<String>().unwrap(),
                                         st.get_value(&citer, 0).get::<String>().unwrap(),
                                         if current_value == true {"enabled"} else {"disabled"});
                }
            }
        }
    }
}

fn set_piter_selected (store: &TreeStore, piter: &gtk::TreeIter) -> bool {
    let mut all_selected = true;
    if let Some(ref citer) = store.iter_children(piter) {
        loop {
            all_selected &= store.get_value(citer, 1).get::<bool>().unwrap();
            if !all_selected || !store.iter_next(citer) { break; }
        }
    }
    store.set_value(piter, 1, &Value::from(&all_selected));
    all_selected
}
