import os.path
import struct

class Register:
    def __init__ (self, name, alias, address):
        self.name, self.address = name, address
        self.alias = name if alias=="_" else alias
        self.changed = False
    
    @staticmethod
    def from_str(s):
        name, alias, address = s.split()
        return Register(name, alias, address)
    
    def __str__ (self):
        alias = "_" if self.alias == self.name else self.alias
        return "{} {} {}\n".format(self.name, alias, self.address)
    
    @property
    def gdbvalue(self):
        inferior = gdb.selected_inferior()
        memory = inferior.read_memory(int(self.address, 0), 4)
        m = struct.unpack("<L", memory)[0]
        return gdb.parse_and_eval(str(m))
    
    def format_value (self, FORMAT):
        value = self.gdbvalue
        int_value = to_unsigned(value, value.type.sizeof)
        try:
            if value.type.code in [gdb.TYPE_CODE_INT, gdb.TYPE_CODE_PTR]:
                if FORMAT == "BIN":
                    value_format = '{{:0{}b}}'.format(8 * value.type.sizeof)
                    fvalue = value_format.format(int_value)
                    fvalue = '_'.join([ fvalue[i:i+8] for i in range(0, len(fvalue), 8) ])
                elif FORMAT == "DECIMAL":
                    value_format = '{}'
                    fvalue = value_format.format(int_value)
                else:
                    value_format = '0x{{:0{}x}}'.format(2 * value.type.sizeof)
                    fvalue = value_format.format(int_value)
                return fvalue
        except (gdb.error, ValueError):
            pass
        return str(value)
    
    def set_value (self, value):
        oldvalue = self.gdbvalue
        if oldvalue.type.code == gdb.TYPE_CODE_INT:
            width = oldvalue.type.sizeof * 8
            if 0 <= value < (2 ** width):
                run("set *{0} = {1}".format(self.address, value))

    @staticmethod
    def find_recursive(rs, name, path, baseaddr):
        for r in rs:
            if r.name == path[0]:
                raddr = format_address(baseaddr + r.address_offset)
                if len(path) == 1:
                    return Register(name, name, raddr)
                else:
                    for f in r.fields:
                        if f.name == path[1]:
                            return Field(name, name, raddr, f.bit_offset, f.bit_width)

class Field (Register):
    def __init__ (self, name, alias, address, boffset, bwidth):
        self.name, self.address, self.boffset, self.bwidth = name, address, boffset, bwidth
        self.alias = name if alias=="_" else alias
        self.changed = False
    
    @staticmethod
    def from_str(s):
        name, alias, address, boffset, bwidth = s.split()
        return Field(name, alias, address, int(boffset), int(bwidth))
    
    def __str__ (self):
        alias = "_" if self.alias == self.name else self.alias
        return "{} {} {} {} {}".format(self.name, alias, self.address, self.boffset, self.bwidth)
    
    def format_value (self, FORMAT):
        value = self.gdbvalue
        try:
            if value.type.code in [gdb.TYPE_CODE_INT, gdb.TYPE_CODE_PTR]:
                int_value = to_unsigned(value, value.type.sizeof)
                int_value = (int_value >> self.boffset) & (0xffff_ffff >> (32 - self.bwidth))
                if FORMAT == "BIN":
                    value_format = '0b{{:0{}b}}'.format(self.bwidth)
                elif FORMAT == "DECIMAL":
                    value_format = '{}'
                else:
                    value_format = '0x{:x}'
                return value_format.format(int_value)
        except (gdb.error, ValueError):
            pass
        return str(value)
    
    def set_value (self, value):
        oldvalue = self.gdbvalue
        if oldvalue.type.code == gdb.TYPE_CODE_INT:
            int_value = to_unsigned(oldvalue, oldvalue.type.sizeof)
            if 0 <= value < (2 ** self.bwidth):
                clean_mask = (0xffff_ffff >> (32 - self.bwidth))
                newvalue = oldvalue & ~(clean_mask << self.boffset) | (value << self.boffset)
                run("set *{0} = {1}".format(self.address, newvalue))


class SvdRegisters (Dashboard.Module):
    """Show the CPU registers and their values."""
    
    FILE = "registers.txt"
    
    def __init__ (self):
        self.table = {}
        self.FORMAT = "HEX"
        self.FORMAT_CHANGED = False
        self.SHOW_CHANGED = False
        
        self.svd_device = None

    def label (self):
        return 'SVD Registers'

    def lines (self, term_width, term_height, style_changed):
        # fetch registers status
        out = []
        registers = []
        if os.path.isfile(SvdRegisters.FILE):
            with open(SvdRegisters.FILE, 'r') as f:
                lines = [l.strip() for l in f.readlines()]
                lines = [l for l in lines if l]
            
            changed_list = []
            for reg_info in lines[1:]:
                # fetch register and update the table
                reg_split = reg_info.split()
                if len(reg_split) == 3:
                    r = Register.from_str(reg_info)
                elif len(reg_split) == 5:
                    r = Field.from_str(reg_info)
                r.value = r.format_value(self.FORMAT)
                old_r = self.table.get(r.alias, None)
                r.changed = old_r and (old_r.value != r.value) and not self.FORMAT_CHANGED
                self.table[r.alias] = r
                registers.append(r)
                if r.changed:
                    changed_list.append((r, old_r))
            # split registers in rows and columns
            max_name = max(len(r.alias) for r in registers)
            max_value = max(len(r.value) for r in registers)
            max_width = max_name + max_value + 2
            per_line = int((term_width + 1) / max_width) or 1
            # redistribute extra space among columns
            extra = int((term_width + 1 - max_width * per_line) / per_line)
            if per_line == 1:
                # center when there is only one column
                max_name += int(extra / 2)
                max_value += int(extra / 2)
            else:
                max_value += extra
            # format registers info
            partial = []
            for r in registers:
                styled_name = ansi(r.alias.rjust(max_name), R.style_low)
                value_style = R.style_selected_1 if r.changed else ''
                styled_value = ansi(r.value.ljust(max_value), value_style)
                partial.append(styled_name + ' ' + styled_value)
            for i in range(0, len(partial), per_line):
                out.append(' '.join(partial[i:i + per_line]).rstrip())
            if changed_list:
                out.append('- '*(term_width//2))
                for r, old_r in changed_list:
                    out.append('{} {} -> {}'.format(ansi(r.alias.rjust(max_name), R.style_low),
                                 ansi(old_r.value, ''), ansi(r.value, '')))
        else:
            raise Exception("{} is missing. Add it".format(SvdRegisters.FILE))

        self.FORMAT_CHANGED = False
        return out
    
    def hex (self, arg):
        self.FORMAT = "HEX"
        self.FORMAT_CHANGED = True
    
    def bin (self, arg):
        self.FORMAT = "BIN"
        self.FORMAT_CHANGED = True
    
    def decimal (self, arg):
        self.FORMAT = "DECIMAL"
        self.FORMAT_CHANGED = True
    
    def changed (self, arg):
        self.SHOW_CHANGED = True
    
    def monitor (self, arg):
        if not self.svd_device:
            try:
                from cmsis_svd.parser import SVDParser
            except:
                raise Exception("Cannot import SVDParser. Check 'cmsis_svd' library installed")
            if os.path.isfile(SvdRegisters.FILE):
                try:
                    with open(SvdRegisters.FILE, 'r') as f:
                        lines = [l.strip() for l in f.readlines()]
                        parser = SVDParser.for_xml_file(lines[0])
                        self.svd_device = parser.get_device()
                except:
                    raise Exception("Cannot load or parse SVD file")
            else:
                raise Exception("{} is missing. Add it".format(SvdRegisters.FILE))
        if self.svd_device and arg:
            args = arg.split()
            name = args[0]
            if name not in self.table:
                r = self.find_register(name)
                if r:
                    r.alias = args[1] if len(args) > 1 else "_"
                    with open(SvdRegisters.FILE, "a") as f:
                        f.write(str(r)+"\n")
                else:
                    raise Exception("Register {} not found".format(name))
            else:
                raise Exception("Register {} already exists".format(name))

    def find_register (self, name):
        path = name.split(".")
        pname = path[0]
        pfound = False
        for p in self.svd_device.peripherals:
            if p.name == pname:
                pfound = True
                if len(path) > 1:
                    return Register.find_recursive(p.registers, name, path[1:], p.base_address)
        if pfound == False:
            raise Exception("Peripheral {} not found".format(pname))

    def remove (self, arg):
        if os.path.isfile(SvdRegisters.FILE):
            with open(SvdRegisters.FILE, 'r') as f:
                lines = f.readlines()
            newlines = [l for l in lines[1:] if arg not in l.split()[:2]]
            with open(SvdRegisters.FILE, 'w') as f:
                f.write(lines[0]+"".join(newlines))
        if arg in self.table:
            del self.table[arg]

    def set_value (self, arg):
        if arg:
            args = arg.split()
            if len(args) == 2:
                name, value = args[0], int(args[1])
                if name in self.table:
                    r = self.table[name]
                    r.set_value(value)
                else:
                    raise Exception("Register {} not found".format(name))

    def commands (self):
        return {
            'hex': {
                'action': self.hex,
                'doc': 'Set hexidemical format.'
            },
            'bin': {
                'action': self.bin,
                'doc': 'Set binary format.'
            },
            'decimal': {
                'action': self.decimal,
                'doc': 'Set decimal format.'
            },
            'changed': {
                'action': self.changed,
                'doc': 'Show old value of changed registers.'
            },
            'add': {
                'action': self.monitor,
                'doc': 'Add register to monitored.'
            },
            'monitor': {
                'action': self.monitor,
                'doc': 'Add register to monitored.'
            },
            'remove': {
                'action': self.remove,
                'doc': 'Remove register from monitored.'
            },
            'set': {
                'action': self.set_value,
                'doc': 'Change register value.'
            },
        }
