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
        
        scrolled_window.set_size_request(500, 600)
        
        grid = Gtk.Grid()
        grid.set_row_spacing(5)
        
        open_button = Gtk.Button(stock = Gtk.STOCK_OPEN)
        ok_button = Gtk.Button(stock = Gtk.STOCK_OK)
        cancel_button = Gtk.Button(stock = Gtk.STOCK_CANCEL)
        open_button.connect("clicked", self.on_open_clicked)
        ok_button.connect("clicked", self.on_ok_clicked)
        cancel_button.connect("clicked", self.on_cancel_clicked)
        
        grid.attach(open_button,     0, 0, 1, 1)
        grid.attach(scrolled_window, 0, 1, 4, 1)
        grid.attach(ok_button,       2, 2, 1, 1)
        grid.attach(cancel_button,   3, 2, 1, 1)
        
        self.add(grid)
        
        if os.path.isfile(FILE):
            with open(FILE, 'r') as f:
                lines = [l.strip() for l in f.readlines()]
            if os.path.isfile(lines[0]):
                self.svd_filename = lines[0]
                self.load_data([l.split()[0] for l in lines[1:]])
            else:
                self.open_file()
        else:
            self.open_file()
            

    def set_piter_selected (self, piter):
        all_selected = True
        citer = self.store.iter_children(piter)
        while citer is not None:
            if self.store[citer][1] == False:
                all_selected = False
                break
            citer = self.store.iter_next(citer)
        self.store[piter][1] = all_selected
        return all_selected
        
    # callback function for the signal emitted by the cellrenderertoggle
    def on_check_toggled(self, widget, path):
        current_value = self.store[path][1]
        self.store[path][1] = not current_value
        current_value = not current_value
        if len(path.split(":")) == 1:
            piter = self.store.get_iter(path)
            citer = self.store.iter_children(piter)
            while citer is not None:
                self.store[citer][1] = current_value
                citer = self.store.iter_next(citer)
            print("{} {}".format(self.store[piter][0], "enabled" if current_value else "disabled"))
        else:
            citer = self.store.get_iter(path)
            piter = self.store.iter_parent(citer)
            all_selected = self.set_piter_selected(piter)
            if all_selected:
                print("{} {}".format(self.store[piter][0], "enabled" if current_value else "disabled"))
            else:
                print("{}.{} {}".format(self.store[piter][0], self.store[citer][0] , "enabled" if current_value else "disabled"))
                
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
    
    def load_data (self, regs=[]):
        parser = SVDParser.for_xml_file(self.svd_filename)
        device = parser.get_device()
        perifs = device.peripherals
        
        # the data are stored in the model
        self.store = Gtk.TreeStore(str, bool, str, str)
        # fill in the model
        for p in perifs:
            paddr = p.base_address
            piter = self.store.append(None, [p.name, False, '0x{:08x}'.format(paddr), p.description.replace('\n', " ")])
            for r in p.registers:
                raddr = paddr + r.address_offset
                name = "{}.{}".format(p.name, r.name)
                self.store.append(piter, [r.name, True if name in regs else False, '0x{:08x}'.format(raddr), r.description.replace('\n', " ")])
            self.set_piter_selected(piter)
        self.view.set_model(self.store)
    
    def on_ok_clicked (self, widget):
        s = self.svd_filename + '\n'
        piter = self.store.get_iter("1")
        while piter is not None:
            citer = self.store.iter_children(piter)
            while citer is not None:
                if self.store[citer][1] == True:
                    s += "{}.{} {}\n".format(self.store[piter][0], self.store[citer][0], self.store[citer][2])
                citer = self.store.iter_next(citer)
            piter = self.store.iter_next(piter)
        with open(FILE, 'w') as f:
            f.write(s)
        self.destroy()
        
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
