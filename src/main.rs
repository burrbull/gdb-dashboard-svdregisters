extern crate gtk;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, FileChooserDialog, TreeView,
         TreeStore, CellRendererText, CellRendererToggle, TreeViewColumn, TreePath, TreeIter, TreeModelExt};

extern crate svd_parser as svd;
use svd::{Register, BitRange};


use std::cell::RefCell;
use std::rc::Rc;
use std::path::{PathBuf, Path};
use std::fs::File;
use std::io::{Read, Write};

use std::collections::HashMap;

const FILE: &str = "registers.txt";


    
fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    
    let window = Window::new(WindowType::Toplevel);
    let view = TreeView::new();
    let open_button = Button::new_with_label("Open");
    let ok_button = Button::new_with_label("Ok");
    let apply_button = Button::new_with_label("Apply");
    let cancel_button = Button::new_with_label("Cancel");
    
    let svd_filename: Rc<RefCell<Option<String>>>    = Rc::new(RefCell::new(None));
    let store:        Rc<RefCell<Option<TreeStore>>> = Rc::new(RefCell::new(None));
    
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
    
    let cell_alias = CellRendererText::new();
    let column_alias = TreeViewColumn::new();
    column_alias.pack_start(&cell_alias, true);
    column_alias.add_attribute(&cell_alias, "text", 4);
    column_alias.add_attribute(&cell_alias, "editable", 5);
    column_alias.set_title("Alias");
    view.append_column(&column_alias);
    
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
    
    scrolled_window.set_size_request(500, 500);
    scrolled_window.set_hexpand(true);
    scrolled_window.set_vexpand(true);
    
    let grid = gtk::Grid::new();
    grid.set_row_spacing(5);
    

    grid.attach(&open_button,     0, 0, 1, 1);
    grid.attach(&scrolled_window, 0, 1, 5, 1);
    grid.attach(&ok_button,       2, 2, 1, 1);
    grid.attach(&apply_button,    3, 2, 1, 1);
    grid.attach(&cancel_button,   4, 2, 1, 1);
    
    window.add(&grid);
         
    window.show_all();
    
    let mut fflag = false;
    let text = &mut String::new();
    {
        let stor = store.clone();
        let svd_f = svd_filename.clone();
        if let Ok(mut f) = File::open(FILE) {
            f.read_to_string(text).expect("Unable to read file");
            let mut lines = text.lines().map(|l| l.trim());
            if let Some(filename) = lines.next() {
                println!("SVD File {}", filename);
                let regs: HashMap<&str, &str> = lines.map(|l| (
                            l.split_whitespace().nth(0).unwrap(),
                            l.split_whitespace().nth(1).unwrap())).collect();
                *stor.borrow_mut() = load_svd(Path::new(&filename));
                *svd_f.borrow_mut() = Some(filename.to_string());
                if let Some(ref st) = *stor.borrow() {
                    fflag = true;
                    view.set_model(st);
                    select_items(&view.clone(), st, regs);
                }
            }
        }
        if !fflag {
            if let Some(pathbuf) = open_file(&window) {
                println!("Open SVD File {:?}", pathbuf);
                *stor.borrow_mut() = load_svd(&pathbuf);
                if let Some(ref st) = *stor.borrow() {
                    *svd_f.borrow_mut() = pathbuf.into_os_string().into_string().ok();
                    view.set_model(st);
                }
            }
        }
        if let Some(ref svd_file) = *svd_filename.borrow() {
            window.set_title(svd_file);
        }
    }
    
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });  
    
    cancel_button.connect_clicked(|_| { gtk::main_quit();  });
    
    {
        let store = store.clone();
        cell_in_out.connect_toggled(move |_,path| {
            if let Some(ref st) = *store.borrow() {
                on_toggle(st, &path)
            }
        });
    }
    {
        let store = store.clone();
        cell_alias.connect_edited(move |_,path,new_text| {
            if let Some(ref st) = *store.borrow() {
                let iter = st.get_iter(&path).unwrap();
                st.set(&iter, &[4], &[&new_text]);
            }
        });
    }
    {
        let store = store.clone();
        let svd_filename = svd_filename.clone();
        ok_button.connect_clicked(move |_| {
            if let Some(ref st) = *store.borrow() {
                if let Some(ref svd_file) = *svd_filename.borrow() {
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
            if let Some(ref st) = *store.borrow() {
                if let Some(ref svd_file) = *svd_filename.borrow() {
                    save_data(st, svd_file).expect("Unable to save file");
                }
            }
        });
    }
    
    {
        let window = window.clone();
        let store = store.clone();
        let svd_filename = svd_filename.clone();
        open_button.connect_clicked(move |_| {
            if let Some(pathbuf) = open_file(&window) {
            println!("Open SVD File {:?}", pathbuf); 
            *store.borrow_mut() = load_svd(&pathbuf);
            if let Some(ref st) = *store.borrow() {
                *svd_filename.borrow_mut() = pathbuf.into_os_string().into_string().ok();
                if let Some(ref svd_file) = *svd_filename.borrow() {
                    window.set_title(svd_file);
                }
                view.set_model(st);
            }
        }
        });
    }
    gtk::main();
}

fn open_file(window: &Window) -> Option<PathBuf> {
    let dialog = FileChooserDialog::new(Some("Please choose a file"), Some(window),
        gtk::FileChooserAction::Open);
    dialog.add_button("Cancel", 0);
    dialog.add_button("Open", 1);
    let response = dialog.run();
    let pathbuf = match response {
        1 => dialog.get_filename(),
        _ =>  None
    };
    dialog.destroy();
    pathbuf
}

fn load_svd (svd_path: &Path) -> Option<TreeStore> {
    let xml = &mut String::new();
    File::open(&svd_path).unwrap().read_to_string(xml).expect("Unable to read file");
    let device = svd::parse(xml);
    let periphs = device.peripherals;
    
    let store = TreeStore::new(&[String::static_type(),
                                 gtk::Type::Bool,
                                 String::static_type(),
                                 String::static_type(),
                                 String::static_type(),
                                 gtk::Type::Bool,
                                 String::static_type(),
                                 String::static_type()
                                 ]);
    for p in &periphs {
        let paddr = p.base_address;
        let pbase = match p.derived_from {
            Some(ref s) => periphs.iter().find(|x| x.name == *s).unwrap_or(p),
            None => p
        };
        let pdesc = pbase.description.to_owned().unwrap_or_default().replace("\n", " ");
        let piter = store.append(None);
        store.set(&piter, &[0, 2, 3], &[&p.name, &format!("0x{:08x}", paddr), &pdesc]);
        if let Some(ref rs) = pbase.registers {
            for reg in rs {
                match reg {
                    &Register::Single(ref r) | &Register::Array(ref r, _) => {
                        let raddr = paddr + r.address_offset;
                        let rdesc = r.description.replace("\n", " ");
                        let riter = store.append(&piter);
                        store.set(&riter, &[0, 2, 3, 5], &[&r.name, &format!("0x{:08x}", raddr), &rdesc, &true]);
                        if let Some(ref fields) = r.fields {
                            for f in fields {
                                let fdesc = f.description.to_owned().unwrap_or_default().replace("\n", " ");
                                let BitRange{ offset: foffset, width: fwidth } = f.bit_range;
                                let fiter = store.append(&riter);
                                store.set(&fiter, &[0, 2, 3, 5, 6, 7], &[&f.name, &format!("0x{:08x}", raddr), &fdesc, &true, &foffset, &fwidth]);
                            }
                        }
                    }
                }
                    
            }
        }
    }
    Some(store)
}

fn select_items (view: &TreeView, store: &TreeStore, regs: HashMap<&str, &str>) {
    if let Some(ref piter) = store.get_iter_first() {
    loop {
        if let Some(ref riter) = store.iter_children(piter) {
        loop {
            find_and_select(view, store, riter, &regs);
            if let Some(ref fiter) = store.iter_children(riter) {
            loop {
                find_and_select(view, store, fiter, &regs);
                if !store.iter_next(fiter) { break; }
            } }
            if !store.iter_next(riter) { break; }
        } }
        set_piter_selected(store, &piter);
        if !store.iter_next(piter)  { break; }
    } }
}

fn find_and_select (view: &TreeView, store: &TreeStore, iter: &TreeIter, regs: &HashMap<&str, &str>) {
    let name = get_reg_name(store, iter);
    if regs.contains_key(&name as &str) {
        store.set(iter, &[1], &[&true]);
        let alias = regs[&name as &str];
        if alias != "_" { 
            store.set(iter, &[4], &[&alias]);
        }
        view.expand_row(&store.get_path(&store.iter_parent(iter).unwrap()).unwrap(), false);
    }
}

fn get_reg_name(store: &TreeStore, citer: &TreeIter) -> String {
    let depth = store.get_path(citer).unwrap().get_depth();
    match depth {
        3 => {
            let fiter = citer;
            let riter = store.iter_parent(&fiter).unwrap();
            let piter = store.iter_parent(&riter).unwrap();
            return format!("{}.{}.{}", store.get_string(&piter, 0),
                                       store.get_string(&riter, 0),
                                       store.get_string(&fiter, 0));
        },
        2 => {
            let riter = citer;
            let piter = store.iter_parent(&riter).unwrap();
            return format!("{}.{}", store.get_string(&piter, 0),
                                    store.get_string(&riter, 0));
        },
        _ => {
            let piter = citer;
            return format!("{}", store.get_string(&piter, 0));
        }
    }
}

fn save_data (store: &TreeStore, svd_file: &String) -> Result<(), std::io::Error> {
    let mut s = svd_file.to_owned() + "\n";
    if let Some(ref piter) = store.get_iter_first() {
    loop {
        if let Some(ref riter) = store.iter_children(piter) {
        loop {
            if store.get_bool(riter, 1) {
                let alias = store.get_string(&riter, 4);
                s += &format!("{} {} {}\n", get_reg_name(store, riter),
                                           if alias != "" {alias} else {"_".to_string()},
                                           store.get_string(&riter, 2));
            }
            if let Some(ref fiter) = store.iter_children(riter) {
            loop {
                if store.get_bool(fiter, 1) {
                    let alias = store.get_string(&fiter, 4);
                    s += &format!("{} {} {} {} {}\n", get_reg_name(store, fiter),
                                               if alias != "" {alias} else {"_".to_string()},
                                               store.get_string(&fiter, 2),
                                               store.get_string(&fiter, 6),
                                               store.get_string(&fiter, 7));
                }
                if !store.iter_next(fiter) { break; }
            } }
            if !store.iter_next(riter) { break; }
        } }
        if !store.iter_next(piter)  { break; }
    } }
    println!("{}", s);
    let mut f = File::create(FILE)?;
    f.write_fmt(format_args!("{}", s))?;
    f.flush()?;
    Ok(())
}

fn on_toggle(st: &TreeStore, path: &TreePath) {
    if let Some(iter) = st.get_iter(path) {
        let current_value = !st.get_bool(&iter, 1);
        st.set(&iter, &[1], &[&current_value]);
        let depth = path.get_depth();
        match depth {
            1 => {
                let piter = iter;
                if let Some(ref riter) = st.iter_children(&piter) {
                loop {
                    st.set(riter, &[1], &[&current_value]);
                    if !st.iter_next(riter) { break; }
                } }
                println!("{} {}", get_reg_name(st, &piter),
                              if current_value == true {"enabled"} else {"disabled"});
            
            },
            2 => {
                let riter = iter;
                if let Some(ref piter) = st.iter_parent(&riter) {
                    let all_selected = set_piter_selected (st, &piter);
                    println!("{} {}", get_reg_name(st, if all_selected {piter} else {&riter}),
                              if current_value == true {"enabled"} else {"disabled"});
                }
            },
            _ => {
                println!("{} {}", get_reg_name(st, &iter),
                              if current_value == true {"enabled"} else {"disabled"});
            }
        }
    }
}


fn set_piter_selected (store: &TreeStore, piter: &TreeIter) -> bool {
    let mut all_selected = true;
    if let Some(ref citer) = store.iter_children(piter) {
        loop {
            all_selected &= store.get_bool(citer, 1);
            if !all_selected || !store.iter_next(citer) { break; }
        }
    }
    store.set(piter, &[1], &[&all_selected]);
    all_selected
}

trait GetValue {
   fn get_bool (&self, iter: &TreeIter, ncol: i32) -> bool;
   fn get_string (&self, iter: &TreeIter, ncol: i32) -> String;
}

impl GetValue for TreeStore {
    fn get_bool (&self, iter: &TreeIter, ncol: i32) -> bool {
        self.get_value(&iter, ncol).get::<bool>().unwrap_or_default()
    }
    fn get_string (&self, iter: &TreeIter, ncol: i32) -> String {
        self.get_value(&iter, ncol).get::<String>().unwrap_or_default()
    }
}
