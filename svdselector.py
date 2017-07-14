import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk
from gi.repository import Pango
import sys
import os.path

FILE = "registers.txt"

from cmsis_svd.parser import SVDParser

class MyWindow(Gtk.ApplicationWindow):
    def __init__(self, app):
        Gtk.Window.__init__(self, title="SVD", application=app)
        self.set_border_width(10)

        self.view = Gtk.TreeView()

        name = Gtk.CellRendererText()
        column_name = Gtk.TreeViewColumn("Name", name, text=0)
        self.view.append_column(column_name)

        renderer_in_out = Gtk.CellRendererToggle()
        column_in_out = Gtk.TreeViewColumn("Out?", renderer_in_out, active=1)
        self.view.append_column(column_in_out)
        renderer_in_out.connect("toggled", self.on_check_toggled)
        
        alias = Gtk.CellRendererText()
        column_alias = Gtk.TreeViewColumn("Alias", alias, text=4, editable=5)
        self.view.append_column(column_alias)
        alias.connect("edited", self.on_alias_edited)

        address = Gtk.CellRendererText()
        column_address = Gtk.TreeViewColumn("Address", address, text=2)
        self.view.append_column(column_address)
        
        description = Gtk.CellRendererText()
        column_description = Gtk.TreeViewColumn("Description", description, text=3)
        self.view.append_column(column_description)

        scrolled_window = Gtk.ScrolledWindow()
        scrolled_window.set_policy(
            Gtk.PolicyType.ALWAYS, Gtk.PolicyType.ALWAYS)
        scrolled_window.add_with_viewport(self.view)
        
        scrolled_window.set_size_request(500, 500)
        scrolled_window.set_hexpand(True)
        scrolled_window.set_vexpand(True)
        
        grid = Gtk.Grid()
        grid.set_row_spacing(5)
        
        open_button = Gtk.Button(stock = Gtk.STOCK_OPEN)
        ok_button = Gtk.Button(stock = Gtk.STOCK_OK)
        apply_button = Gtk.Button(stock = Gtk.STOCK_APPLY)
        cancel_button = Gtk.Button(stock = Gtk.STOCK_CANCEL)
        open_button.connect("clicked", self.on_open_clicked)
        ok_button.connect("clicked", self.on_ok_clicked)
        apply_button.connect("clicked", self.on_apply_clicked)
        cancel_button.connect("clicked", self.on_cancel_clicked)
        
        grid.attach(open_button,     0, 0, 1, 1)
        grid.attach(scrolled_window, 0, 1, 5, 1)
        grid.attach(ok_button,       2, 2, 1, 1)
        grid.attach(apply_button,    3, 2, 1, 1)
        grid.attach(cancel_button,   4, 2, 1, 1)
        
        self.add(grid)
        
        if os.path.isfile(FILE):
            with open(FILE, 'r') as f:
                lines = [l.strip() for l in f.readlines()]
            if os.path.isfile(lines[0]):
                self.svd_filename = lines[0]
                self.load_data(dict([l.split()[:2] for l in lines[1:]]))
            else:
                self.open_file()
        else:
            self.open_file()
            

    def set_piter_selected (self, piter):
        all_selected = True
        riter = self.store.iter_children(piter)
        while riter is not None:
            if self.store[riter][1] == False:
                all_selected = False
                break
            riter = self.store.iter_next(riter)
        self.store[piter][1] = all_selected
        return all_selected
        
    # callback function for the signal emitted by the cellrenderertoggle
    def on_check_toggled(self, widget, path):
        current_value = self.store[path][1]
        self.store[path][1] = not current_value
        current_value = not current_value
        citer = self.store.get_iter(path)
        print_iter = citer
        lenpath = len(path.split(":"))
        if lenpath == 1:
            piter = citer
            riter = self.store.iter_children(piter)
            while riter is not None:
                self.store[riter][1] = current_value
                riter = self.store.iter_next(riter)
        elif lenpath == 2:
            riter = citer
            piter = self.store.iter_parent(riter)
            all_selected = self.set_piter_selected(piter)
            if all_selected:
                print_iter = piter
        print("{} {}".format(self.get_reg_name(print_iter), "enabled" if current_value else "disabled"))
    
    def get_reg_name(self, citer):
        lenpath = len(str(self.store.get_path(citer)).split(":"))
        if lenpath == 3:
            fiter = citer
            riter = self.store.iter_parent(fiter)
            piter = self.store.iter_parent(riter)
            return "{}.{}.{}".format(self.store[piter][0], self.store[riter][0], self.store[fiter][0])
        elif lenpath == 2:
            riter = citer
            piter = self.store.iter_parent(riter)
            return "{}.{}".format(self.store[piter][0], self.store[riter][0])
        else:
            piter = citer
            return "{}".format(self.store[piter][0])
    
    def on_alias_edited(self, widget, path, new_text):
        self.store[path][4] = new_text
        
    def on_open_clicked (self, widget):
        self.open_file()
    
    def open_file (self):
        dialog = Gtk.FileChooserDialog("Please choose a file", self,
            Gtk.FileChooserAction.OPEN,
            (Gtk.STOCK_CANCEL, Gtk.ResponseType.CANCEL,
             Gtk.STOCK_OPEN, Gtk.ResponseType.OK))
        response = dialog.run()
        if response == Gtk.ResponseType.OK:
            self.svd_filename = dialog.get_filename()
            self.load_data()
        elif response == Gtk.ResponseType.CANCEL:
            print("Cancel clicked")
        dialog.destroy()
    
    def load_data (self, regs={}):
        parser = SVDParser.for_xml_file(self.svd_filename)
        device = parser.get_device()
        perifs = device.peripherals
        
        # the data are stored in the model
        self.store = Gtk.TreeStore(str, bool, str, str, str, bool, str, str)
        # fill in the model
        for p in perifs:
            paddr = p.base_address
            piter = self.store.append(None, [p.name, False, '0x{:08x}'.format(paddr), p.description.replace('\n', " "), "", False, "", ""])
            for r in p.registers:
                raddr = paddr + r.address_offset
                riter = self.store.append(piter, [r.name, False, '0x{:08x}'.format(raddr), r.description.replace('\n', " "), "", True, "", ""])
                for f in r.fields:
                    self.store.append(riter, [f.name, False, '0x{:08x}'.format(raddr), f.description.replace('\n', " "), "", True, str(f.bit_offset), str(f.bit_width)])
        self.set_title(self.svd_filename)
        self.view.set_model(self.store)
        self.select_items(regs)
    
    def select_items(self, regs):
        piter = self.store.get_iter_first()
        while piter is not None:
            riter = self.store.iter_children(piter)
            while riter is not None:
                self.find_and_select(riter, regs)
                fiter = self.store.iter_children(riter)
                while fiter is not None:
                    self.find_and_select(fiter, regs)
                    fiter = self.store.iter_next(fiter)
                riter = self.store.iter_next(riter)
            self.set_piter_selected(piter)
            piter = self.store.iter_next(piter)
    
    def find_and_select(self, citer, regs):
        name = self.get_reg_name(citer)
        if name in regs:
            self.store[citer][1] = True
            self.store[citer][4] = regs[name] if regs[name] != "_" else ""
            self.view.expand_row(self.store.get_path(self.store.iter_parent(citer)), False)
    
    def save_data (self):
        s = self.svd_filename + '\n'
        piter = self.store.get_iter("1")
        while piter is not None:
            riter = self.store.iter_children(piter)
            while riter is not None:
                if self.store[riter][1] == True:
                    s += "{} {} {}\n".format(self.get_reg_name(riter), self.store[riter][4] or "_", self.store[riter][2])
                fiter = self.store.iter_children(riter)
                while fiter is not None:
                    if self.store[fiter][1] == True:
                        s += "{} {} {} {} {}\n".format(self.get_reg_name(fiter), self.store[fiter][4] or "_", self.store[riter][2], self.store[fiter][6], self.store[fiter][7])
                    fiter = self.store.iter_next(fiter)
                riter = self.store.iter_next(riter)
            piter = self.store.iter_next(piter)
        with open(FILE, 'w') as f:
            f.write(s)
    
    def on_ok_clicked (self, widget):
        self.save_data()
        self.destroy()
        
    def on_apply_clicked (self, widget):
        self.save_data()
        
    def on_cancel_clicked (self, widget):
        self.destroy()

class MyApplication(Gtk.Application):

    def __init__(self):
        Gtk.Application.__init__(self)

    def do_activate(self):
        win = MyWindow(self)
        win.show_all()

    def do_startup(self):
        Gtk.Application.do_startup(self)

app = MyApplication()
exit_status = app.run(sys.argv)
sys.exit(exit_status)
