import os.path

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

    def lines (self, term_width, style_changed):
        # fetch registers status
        out = []
        registers = []
        if os.path.isfile(SvdRegisters.FILE):
            with open(SvdRegisters.FILE, 'r') as f:
                lines = [l.strip() for l in f.readlines()]
            
            changed_list = []
            for reg_info in lines[1:]:
                # fetch register and update the table
                reg_split = reg_info.split()
                svdname, name, address = reg_split[0], reg_split[1], reg_split[2]
                if len(reg_split) == 5:
                    bit_offset, bit_width = int(reg_split[3]), int(reg_split[4])
                else:
                    bit_offset, bit_width = None, None
                if name == "_": name = svdname
                value = gdb.parse_and_eval("*"+address)
                string_value = self.format_value(value, bit_offset, bit_width)
                old_value = self.table.get(name, '')
                changed = self.table and (old_value != string_value) and not self.FORMAT_CHANGED
                self.table[name] = string_value
                registers.append((name, string_value, changed))
                if changed:
                    changed_list.append((name, old_value, string_value))
            # split registers in rows and columns, each column is composed of name,
            # space, value and another trailing space which is skipped in the last
            # column (hence term_width + 1)
            max_name = max(len(name) for name, _, _ in registers)
            max_value = max(len(value) for _, value, _ in registers)
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
            for name, value, changed in registers:
                styled_name = ansi(name.rjust(max_name), R.style_low)
                value_style = R.style_selected_1 if changed else ''
                styled_value = ansi(value.ljust(max_value), value_style)
                partial.append(styled_name + ' ' + styled_value)
            for i in range(0, len(partial), per_line):
                out.append(' '.join(partial[i:i + per_line]).rstrip())
            if changed_list:
                out.append('- '*(term_width//2))
                for name, old, new in changed_list:
                    out.append('{} {} -> {}'.format(ansi(name.rjust(max_name), R.style_low),
                                 ansi(old, ''), ansi(new, '')))
                                 
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
                if os.path.isfile(SvdRegisters.FILE):
                    with open(SvdRegisters.FILE, 'r') as f:
                        lines = [l.strip() for l in f.readlines()]
                        parser = SVDParser.for_xml_file(lines[0])
                        self.svd_device = parser.get_device()
            except:
                raise Exception("Cannot open or parse SVD file. Check 'cmsis_svd' library installed")
        if self.svd_device and arg:
            args = arg.split()
            name = args[0]
            res = self.find_register(name)
            if res:
                address, boffset, bwidth = res
                alias = args[1] if len(args) > 1 else "_"
                b = " {} {}".format(boffset, bwidth) if boffset else ""
                line = "{} {} {} {}\n".format(name, alias, address, b)
                with open(SvdRegisters.FILE, "a") as f:
                    f.write(line)
            else:
                raise Exception("Register {} is absent".format(name))
    
    def remove (self, arg):
        if os.path.isfile(SvdRegisters.FILE):
            with open(SvdRegisters.FILE, 'r') as f:
                lines = f.readlines()
            newlines = [l for l in lines[1:] if arg not in l.split()[:2]]
            with open(SvdRegisters.FILE, 'w') as f:
                f.write(lines[0]+"".join(newlines))
    
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
            'monitor': {
                'action': self.monitor,
                'doc': 'Add register to monitored.'
            },
            'remove': {
                'action': self.remove,
                'doc': 'Remove register from monitored.'
            },
        }
    
    def format_value (self, value, boffset, bwidth):
        try:
            if value.type.code in [gdb.TYPE_CODE_INT, gdb.TYPE_CODE_PTR]:
                int_value = to_unsigned(value, value.type.sizeof)
                if bwidth:
                    int_value = (int_value >> boffset) - (int_value>>(boffset+bwidth)<<bwidth)
                    if self.FORMAT == "BIN":
                        value_format = '0b{{:0{}b}}'.format(bwidth)
                    elif self.FORMAT == "DECIMAL":
                        value_format = '{}'
                    else:
                        value_format = '0x{:x}'
                    return value_format.format(int_value)
                else:
                    if self.FORMAT == "BIN":
                        value_format = '{{:0{}b}}'.format(8 * value.type.sizeof)
                        fvalue = value_format.format(int_value)
                        fvalue = '_'.join([ fvalue[i:i+8] for i in range(0, len(fvalue), 8) ])
                    elif self.FORMAT == "DECIMAL":
                        value_format = '{}'
                        fvalue = value_format.format(int_value)
                    else:
                        value_format = '0x{{:0{}x}}'.format(2 * value.type.sizeof)
                        fvalue = value_format.format(int_value)
                    return fvalue
        except (gdb.error, ValueError):
            # convert to unsigned but preserve code and flags information
            pass
        return str(value)
    
    def find_register (self, name):
        path = name.split(".")
        if len(path) > 1:
            for p in self.svd_device.peripherals:
                if p.name == path[0]:
                    for r in p.registers:
                        if r.name == path[1]:
                            raddr = '0x{:08x}'.format(p.base_address + r.address_offset)
                            if len(path) == 2:
                                return raddr, None, None
                            else:
                                for f in r.fields:
                                    if f.name == path[2]:
                                        return raddr, f.bit_offset, f.bit_width
