//#![windows_subsystem = "windows"]

use gtk::prelude::*;
use gtk::{
    Button, CellRendererText, CellRendererToggle, TreeIter, TreePath, TreeStore, TreeView,
    TreeViewColumn,
};

use svd::{Cluster, EnumeratedValues, Field, Register, RegisterCluster, RegisterInfo};
use svd_parser::svd;

use std::{
    cell::RefCell,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use std::collections::HashMap;

const FILE: &str = "registers.txt";

fn rm_white(text: &str) -> String {
    use lazy_regex::regex;
    let re = regex!(r"\s\s+");
    re.replace_all(text, " ").to_string()
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let view = TreeView::new();
    let open_button = Button::with_label("Open");
    let ok_button = Button::with_label("Ok");
    let apply_button = Button::with_label("Apply");
    let cancel_button = Button::with_label("Cancel");

    let svd_filename: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let store: Rc<RefCell<Option<TreeStore>>> = Rc::new(RefCell::new(None));

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
    column_in_out.add_attribute(&cell_in_out, "activatable", 5);
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

    let scrolled_window = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    scrolled_window.set_policy(gtk::PolicyType::Always, gtk::PolicyType::Always);
    //scrolled_window.add_with_viewport(&view);
    scrolled_window.add(&view);

    scrolled_window.set_size_request(500, 500);
    scrolled_window.set_hexpand(true);
    scrolled_window.set_vexpand(true);

    let grid = gtk::Grid::new();
    grid.set_row_spacing(5);

    view.set_tooltip_column(8);

    grid.attach(&open_button, 0, 0, 1, 1);
    grid.attach(&scrolled_window, 0, 1, 5, 1);
    grid.attach(&ok_button, 2, 2, 1, 1);
    grid.attach(&apply_button, 3, 2, 1, 1);
    grid.attach(&cancel_button, 4, 2, 1, 1);

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
                let regs: HashMap<&str, &str> = lines
                    .filter(|l| !l.is_empty())
                    .map(|l| {
                        let mut ws = l.split_whitespace();
                        (ws.next().unwrap(), ws.next().unwrap())
                    })
                    .collect();
                *stor.borrow_mut() = load_svd(Path::new(&filename)).unwrap();
                *svd_f.borrow_mut() = Some(filename.to_string());
                if let Some(st) = &*stor.borrow() {
                    fflag = true;
                    view.set_model(Some(st));
                    select_items(&view.clone(), st, &regs);
                }
            }
        }
        if !fflag {
            if let Some(pathbuf) = choose_file(&window) {
                println!("Open SVD File {:?}", pathbuf);
                *stor.borrow_mut() = load_svd(&pathbuf).unwrap();
                if let Some(st) = &*stor.borrow() {
                    *svd_f.borrow_mut() = pathbuf.into_os_string().into_string().ok();
                    view.set_model(Some(st));
                }
            }
        }
        if let Some(svd_file) = &*svd_filename.borrow() {
            window.set_title(svd_file);
        }
    }

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    cancel_button.connect_clicked(|_| {
        gtk::main_quit();
    });

    {
        let store = store.clone();
        cell_in_out.connect_toggled(move |_, path| {
            if let Some(st) = &*store.borrow() {
                on_toggle(st, &path)
            }
        });
    }
    {
        let store = store.clone();
        cell_alias.connect_edited(move |_, path, new_text| {
            if let Some(st) = &*store.borrow() {
                let iter = st.iter(&path).unwrap();
                st.set_value(&iter, 4, &new_text.to_value());
            }
        });
    }
    {
        let store = store.clone();
        let svd_filename = svd_filename.clone();
        ok_button.connect_clicked(move |_| {
            if let Some(st) = &*store.borrow() {
                if let Some(svd_file) = &*svd_filename.borrow() {
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
            if let Some(st) = &*store.borrow() {
                if let Some(svd_file) = &*svd_filename.borrow() {
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
            if let Some(pathbuf) = choose_file(&window) {
                println!("Open SVD File {:?}", pathbuf);
                *store.borrow_mut() = load_svd(&pathbuf).unwrap();
                if let Some(st) = &*store.borrow() {
                    *svd_filename.borrow_mut() = pathbuf.into_os_string().into_string().ok();
                    if let Some(svd_file) = &*svd_filename.borrow() {
                        window.set_title(svd_file);
                    }
                    view.set_model(Some(st));
                }
            }
        });
    }
    gtk::main();
}

fn choose_file(window: &gtk::Window) -> Option<PathBuf> {
    let dialog = gtk::FileChooserDialog::with_buttons(
        Some("Please choose a file"),
        Some(window),
        gtk::FileChooserAction::Open,
        &[
            ("Cancel", gtk::ResponseType::Cancel),
            ("Open", gtk::ResponseType::Ok),
        ],
    );
    let response = dialog.run();
    let pathbuf = match response {
        okr if okr == gtk::ResponseType::Ok.into() => dialog.filename(),
        _ => None,
    };
    dialog.close();
    pathbuf
}

use indexmap::IndexMap;
use std::iter::FromIterator;

fn load_svd(svd_path: &Path) -> Result<Option<TreeStore>, anyhow::Error> {
    let xml = &mut String::new();
    File::open(&svd_path)
        .unwrap()
        .read_to_string(xml)
        .expect("Unable to read file");
    let device = svd_parser::parse(xml)?;

    let permap =
        IndexMap::<&str, _>::from_iter(device.peripherals.iter().map(|i| (i.name.as_str(), i)));

    fn add_reg_ev<'a>(
        ev_map: &mut HashMap<String, &'a EnumeratedValues>,
        reg: &'a Register,
        regpath: &String,
    ) {
        match reg {
            Register::Single(r) | Register::Array(r, _) => {
                if let Some(fields) = &r.fields {
                    for f in fields {
                        for evalues in &f.enumerated_values {
                            if let Some(ev_name) = &evalues.name {
                                ev_map.insert(
                                    format!("{}.{}.{}.{}", regpath, r.name, f.name, ev_name),
                                    &evalues,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_clus_ev<'a>(
        ev_map: &mut HashMap<String, &'a EnumeratedValues>,
        reg: &'a Cluster,
        cpath: &String,
    ) {
        match reg {
            Cluster::Single(c) | Cluster::Array(c, _) => {
                for rc in &c.children {
                    match rc {
                        RegisterCluster::Register(reg) => {
                            add_reg_ev(ev_map, reg, &format!("{}.{}", cpath, c.name))
                        }
                        RegisterCluster::Cluster(cl) => {
                            add_clus_ev(ev_map, cl, &format!("{}.{}", cpath, c.name))
                        }
                    }
                }
            }
        }
    }

    let mut ev_map = HashMap::<String, &EnumeratedValues>::new();
    for (pname, p) in &permap {
        if let Some(rs) = &p.registers {
            for rc in rs {
                match rc {
                    RegisterCluster::Register(reg) => {
                        add_reg_ev(&mut ev_map, reg, &pname.to_string())
                    }
                    RegisterCluster::Cluster(cl) => {
                        add_clus_ev(&mut ev_map, cl, &pname.to_string())
                    }
                }
            }
        }
    }

    let store = TreeStore::new(&[
        String::static_type(), // name
        bool::static_type(),   // active
        String::static_type(), // address
        String::static_type(), // description
        String::static_type(), // alias
        bool::static_type(),   // sens
        String::static_type(), // offset
        String::static_type(), // width
        String::static_type(), // tooltip
        String::static_type(), // path
        String::static_type(), // type
    ]);
    for (pname, p) in &permap {
        let paddr = p.base_address;
        let pbase = match &p.derived_from {
            Some(s) => permap.get(s.as_str()).unwrap_or(p),
            None => p,
        };
        let pdesc = rm_white(&pbase.description.to_owned().unwrap_or_default());
        let piter = store.append(None);
        store.set(
            &piter,
            &[
                (0, &pname),
                (2, &format!("0x{:08x}", paddr)),
                (3, &pdesc),
                (9, &pname),
                (10, &"p"),
            ],
        );
        let ptooltip = if **pname != pbase.name {
            format!(
                "<b>{}</b>\n  derived from: <i>{}</i>\n{}",
                pname, pbase.name, &pdesc
            )
        } else {
            format!("<b>{}</b>\n{}", pname, &pdesc)
        };
        store.set_value(&piter, 8, &ptooltip.to_value());
        if let Some(rcs) = &pbase.registers {
            for rc in rcs {
                let rciter = &store.append(Some(&piter));
                let path = &pname.to_string();
                let derpath = &pbase.name.to_string();
                match rc {
                    RegisterCluster::Register(reg) => {
                        add_register_tree(&store, rciter, &ev_map, reg, &path, &derpath, paddr)
                    }
                    RegisterCluster::Cluster(cl) => {
                        add_cluster_tree(&store, rciter, &ev_map, cl, &path, &derpath, paddr)
                    }
                }
            }
        }
    }
    Ok(Some(store))
}

fn add_cluster_tree(
    store: &TreeStore,
    citer: &TreeIter,
    ev_map: &HashMap<String, &EnumeratedValues>,
    c: &Cluster,
    cpath: &String,
    derpath: &String,
    baseaddr: u64,
) {
    let caddr = baseaddr + c.address_offset as u64;
    let desc = rm_white(c.description.as_deref().unwrap_or(""));
    let path = format!("{}.{}", cpath, c.name);
    let derpath = format!("{}.{}", derpath, c.name);
    match c {
        Cluster::Single(c) => {
            store.set(
                citer,
                &[
                    (0, &c.name),
                    (2, &format!("0x{:08x}", caddr)),
                    (3, &desc),
                    (5, &false),
                    (9, &path),
                    (10, &"c"),
                ],
            );
            store.set_value(
                citer,
                8,
                &format!(
                    "<b>{} cluster </b>\n  offset: 0x{:02x}\n{}",
                    &path, c.address_offset, &desc
                )
                .to_value(),
            );
            for rc in &c.children {
                let rciter = &store.append(Some(citer));
                match rc {
                    RegisterCluster::Register(reg) => {
                        add_register_tree(&store, rciter, &ev_map, reg, &path, &derpath, caddr)
                    }
                    RegisterCluster::Cluster(cl) => {
                        add_cluster_tree(&store, rciter, &ev_map, cl, &path, &derpath, caddr)
                    }
                }
            }
        }
        Cluster::Array(c, dim) => {
            store.set(
                citer,
                &[
                    (0, &c.name),
                    (3, &desc),
                    (5, &false),
                    (9, &path),
                    (10, &"ca"),
                ],
            );
            store.set_value(
                citer,
                8,
                &format!("<b>{} cluster array</b>\n{}", &path, &desc).to_value(),
            );
            for (i, idx) in dim.indexes().enumerate() {
                let offset = dim.dim_increment * (i as u32);
                let citer = &store.append(Some(citer));
                let cname = c.name.replace("[%s]", &idx).replace("%s", &idx);
                let desc = desc.replace("%s", &idx);
                let path = format!("{}.{}", cpath, cname);
                store.set(
                    citer,
                    &[
                        (0, &cname),
                        (2, &format!("0x{:08x}", caddr + offset as u64)),
                        (3, &desc),
                        (5, &false),
                        (9, &path),
                        (10, &"c"),
                    ],
                );
                store.set_value(
                    citer,
                    8,
                    &format!(
                        "<b>{}</b>\n  offset: 0x{:02x}\n{}",
                        &path,
                        c.address_offset + offset,
                        &desc
                    )
                    .to_value(),
                );
                for rc in &c.children {
                    let rciter = &store.append(Some(citer));
                    match rc {
                        RegisterCluster::Register(reg) => add_register_tree(
                            &store,
                            rciter,
                            &ev_map,
                            reg,
                            &path,
                            &derpath,
                            caddr + offset as u64,
                        ),
                        RegisterCluster::Cluster(cl) => add_cluster_tree(
                            &store,
                            rciter,
                            &ev_map,
                            cl,
                            &path,
                            &derpath,
                            caddr + offset as u64,
                        ),
                    }
                }
            }
        }
    }
}

fn add_register_tree(
    store: &TreeStore,
    riter: &TreeIter,
    ev_map: &HashMap<String, &EnumeratedValues>,
    r: &Register,
    rpath: &String,
    derpath: &String,
    baseaddr: u64,
) {
    let raddr = baseaddr + r.address_offset as u64;
    let rdesc = rm_white(r.description.as_deref().unwrap_or(""));
    let path = format!("{}.{}", rpath, r.name);
    match r {
        Register::Single(r) => {
            store.set(
                riter,
                &[
                    (0, &r.name),
                    (2, &format!("0x{:08x}", raddr)),
                    (3, &rdesc),
                    (5, &true),
                    (9, &path),
                    (10, &"r"),
                ],
            );
            store.set_value(
                riter,
                8,
                &format!(
                    "<b>{} register </b>\n  offset: 0x{:02x}\n{}",
                    &path, r.address_offset, &rdesc
                )
                .to_value(),
            );

            add_fields_tree(store, riter, ev_map, r, &path, derpath, raddr as u64);
        }
        Register::Array(r, dim) => {
            store.set(
                riter,
                &[
                    (0, &r.name),
                    (3, &rdesc),
                    (5, &false),
                    (9, &path),
                    (10, &"ra"),
                ],
            );
            store.set_value(
                riter,
                8,
                &format!("<b>{} register array</b>\n{}", &path, &rdesc).to_value(),
            );

            for (i, idx) in dim.indexes().enumerate() {
                let offset = dim.dim_increment * (i as u32);
                let riter = &store.append(Some(riter));
                let rname = r.name.replace("[%s]", &idx).replace("%s", &idx);
                let rdesc = rdesc.replace("%s", &idx);
                let path = format!("{}.{}", rpath, rname);
                store.set(
                    riter,
                    &[
                        (0, &rname),
                        (2, &format!("0x{:08x}", raddr + offset as u64)),
                        (3, &rdesc),
                        (5, &true),
                        (9, &path),
                        (10, &"r"),
                    ],
                );
                store.set_value(
                    riter,
                    8,
                    &format!(
                        "<b>{}</b>\n  offset: 0x{:02x}\n{}",
                        &path,
                        r.address_offset + offset,
                        &rdesc
                    )
                    .to_value(),
                );
                add_fields_tree(
                    store,
                    riter,
                    ev_map,
                    r,
                    &path,
                    derpath,
                    raddr + offset as u64,
                );
            }
        }
    }
}

fn add_fields_tree(
    store: &TreeStore,
    riter: &TreeIter,
    ev_map: &HashMap<String, &EnumeratedValues>,
    r: &RegisterInfo,
    path: &String,
    derpath: &String,
    raddr: u64,
) {
    if let Some(fields) = &r.fields {
        for f in fields {
            let mut svalues = String::new();
            for evalues in &f.enumerated_values {
                if let Some(evs_name) = &evalues.derived_from {
                    svalues.push_str(&format!("\n derived from: <i>{}</i>", evs_name));
                }
                let de = match &evalues.derived_from {
                    Some(evs_name) => {
                        let derived_path: Vec<&str> = evs_name.split(".").collect();
                        match derived_path.len() {
                            4 => ev_map.get(evs_name).unwrap(),
                            3 => ev_map.get(&format!("{}.{}", derpath, evs_name)).unwrap(),
                            1 => {
                                let fname = fields
                                    .iter()
                                    .map(|f| &f.name)
                                    .find(|n| {
                                        ev_map.contains_key(&format!(
                                            "{}.{}.{}.{}",
                                            derpath, r.name, n, evs_name
                                        ))
                                    })
                                    .unwrap();
                                ev_map
                                    .get(&format!("{}.{}.{}.{}", derpath, r.name, fname, evs_name))
                                    .unwrap()
                            }
                            _ => unimplemented!(),
                        }
                    }
                    None => evalues,
                };
                for ev in &de.values {
                    if let Some(val) = ev.value {
                        svalues.push_str(&format!("\n\t{} : {}", val, ev.name));
                    }
                }
            }

            let fdesc = rm_white(f.description.as_deref().unwrap_or(""));
            let fpath = format!("{}.{}", path, f.name);
            let br = f.bit_range;
            let fiter = store.append(Some(riter));

            match f {
                Field::Single(f) => {
                    let offset = br.offset;
                    let width = br.width;
                    store.set(
                        &fiter,
                        &[
                            (0, &f.name),
                            (2, &format!("0x{:08x}", raddr)),
                            (
                                3,
                                &format!("[{}-{}]: {}", offset + width - 1, offset, &fdesc),
                            ),
                            (5, &true),
                            (6, &offset),
                            (7, &width),
                            (9, &fpath),
                            (10, &"f"),
                        ],
                    );

                    store.set_value(
                        &fiter,
                        8,
                        &format!(
                            "<b>{} field</b>\n [{}-{}]: {}{}{}",
                            &fpath,
                            offset + width - 1,
                            offset,
                            &fdesc,
                            (if !svalues.is_empty() { "\nValues:" } else { "" }),
                            &svalues
                        )
                        .to_value(),
                    );
                }
                Field::Array(f, dim) => {
                    let width = br.width;
                    store.set(
                        &fiter,
                        &[
                            (0, &f.name),
                            (3, &fdesc),
                            (5, &false),
                            (9, &fpath),
                            (10, &"fa"),
                        ],
                    );
                    store.set_value(
                        &fiter,
                        8,
                        &format!("<b>{} field array</b>\n{}", &fpath, &fdesc).to_value(),
                    );
                    for (i, idx) in dim.indexes().enumerate() {
                        let fiter = &store.append(Some(&fiter));
                        let offset = dim.dim_increment * (i as u32);
                        let fname = f.name.replace("[%s]", &idx).replace("%s", &idx);
                        let fdesc = fdesc.replace("%s", &idx);
                        let fpath = format!("{}.{}", path, fname);

                        store.set(
                            &fiter,
                            &[
                                (0, &fname),
                                (2, &format!("0x{:08x}", raddr)),
                                (
                                    3,
                                    &format!("[{}-{}]: {}", offset + width - 1, offset, &fdesc),
                                ),
                                (5, &true),
                                (6, &offset),
                                (7, &width),
                                (9, &fpath),
                                (10, &"f"),
                            ],
                        );

                        store.set_value(
                            &fiter,
                            8,
                            &format!(
                                "<b>{} field</b>\n [{}-{}]: {}{}{}",
                                &fpath,
                                offset + width - 1,
                                offset,
                                &fdesc,
                                (if !svalues.is_empty() { "\nValues:" } else { "" }),
                                &svalues
                            )
                            .to_value(),
                        );
                    }
                }
            }
        }
    }
}

fn recursive_load(view: &TreeView, store: &TreeStore, iter: &TreeIter, regs: &HashMap<&str, &str>) {
    if let Some(iter) = &store.iter_children(Some(iter)) {
        loop {
            find_and_select(view, store, iter, regs);
            recursive_load(view, store, iter, regs);
            if !store.iter_next(iter) {
                break;
            }
        }
    }
}

fn select_items(view: &TreeView, store: &TreeStore, regs: &HashMap<&str, &str>) {
    if let Some(iter) = &store.iter_first() {
        loop {
            recursive_load(view, store, iter, regs);
            if !store.iter_next(iter) {
                break;
            }
        }
    }
}

fn find_and_select(
    view: &TreeView,
    store: &TreeStore,
    iter: &TreeIter,
    regs: &HashMap<&str, &str>,
) {
    let name = get_reg_path(store, iter);
    if regs.contains_key(&name as &str) {
        store.set_value(iter, 1, &true.to_value());
        let alias = regs[&name as &str];
        if alias != "_" {
            store.set_value(iter, 4, &alias.to_value());
        }
        view.expand_to_path(&store.path(iter).unwrap());
    }
}

fn get_reg_path(store: &TreeStore, citer: &TreeIter) -> String {
    store.get_string(&citer, 9)
}

fn recursive_save(store: &TreeStore, iter: &TreeIter, s: &mut String) {
    if let Some(iter) = &store.iter_children(Some(iter)) {
        loop {
            if store.get_bool(iter, 1) {
                let alias = store.get_string(&iter, 4);
                match store.get_string(&iter, 10).as_str() {
                    "r" => {
                        *s += &format!(
                            "{} {} {}\n",
                            get_reg_path(store, iter),
                            if !alias.is_empty() {
                                alias
                            } else {
                                "_".to_string()
                            },
                            store.get_string(&iter, 2)
                        );
                    }
                    "f" => {
                        *s += &format!(
                            "{} {} {} {} {}\n",
                            get_reg_path(store, iter),
                            if !alias.is_empty() {
                                alias
                            } else {
                                "_".to_string()
                            },
                            store.get_string(&iter, 2),
                            store.get_string(&iter, 6),
                            store.get_string(&iter, 7)
                        );
                    }
                    _ => {}
                }
            }
            recursive_save(store, iter, s);
            if !store.iter_next(iter) {
                break;
            }
        }
    }
}

fn save_data(store: &TreeStore, svd_file: &String) -> Result<(), std::io::Error> {
    let mut s = svd_file.clone() + "\n";
    if let Some(piter) = &store.iter_first() {
        loop {
            recursive_save(store, piter, &mut s);
            if !store.iter_next(piter) {
                break;
            }
        }
    }
    println!("{}", s);
    let mut f = File::create(FILE)?;
    f.write_fmt(format_args!("{}", s))?;
    f.flush()?;
    Ok(())
}

fn on_toggle(st: &TreeStore, path: &TreePath) {
    if let Some(iter) = st.iter(path) {
        let current_value = !st.get_bool(&iter, 1);
        st.set_value(&iter, 1, &current_value.to_value());
        println!(
            "{} {}",
            get_reg_path(st, &iter),
            if current_value { "enabled" } else { "disabled" }
        );
    }
}

trait GetValue {
    fn get_bool(&self, iter: &TreeIter, ncol: i32) -> bool;
    fn get_string(&self, iter: &TreeIter, ncol: i32) -> String;
}

impl GetValue for TreeStore {
    fn get_bool(&self, iter: &TreeIter, ncol: i32) -> bool {
        self.value(&iter, ncol).get::<bool>().unwrap_or_default()
    }
    fn get_string(&self, iter: &TreeIter, ncol: i32) -> String {
        self.value(&iter, ncol).get::<String>().unwrap_or_default()
    }
}
