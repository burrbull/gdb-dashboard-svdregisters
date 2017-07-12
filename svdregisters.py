import os.path

class SvdRegisters(Dashboard.Module):
    """Show the CPU registers and their values."""
    
    FILE = "registers.txt"
    
    def __init__(self):
        self.table = {}
        self.FORMAT = "HEX"
        self.FORMAT_CHANGED = False
        self.SHOW_CHANGED = False

    def label(self):
        return 'SVD Registers'

    def lines(self, term_width, style_changed):
        # fetch registers status
        out = []
        registers = []
        if os.path.isfile(SvdRegisters.FILE):
            with open(SvdRegisters.FILE, 'r') as f:
                lines = [l.strip() for l in f.readlines()]
            
            changed_list = []
            for reg_info in lines[1:]:
                # fetch register and update the table
                name, address = reg_info.split()
                
                value = gdb.parse_and_eval("*"+address)
                string_value = self.format_value(value)
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
    
    def hex(self, arg):
        self.FORMAT = "HEX"
        self.FORMAT_CHANGED = True
    
    def bin(self, arg):
        self.FORMAT = "BIN"
        self.FORMAT_CHANGED = True
    
    def decimal(self, arg):
        self.FORMAT = "DECIMAL"
        self.FORMAT_CHANGED = True
    
    def changed(self, arg):
        self.SHOW_CHANGED = True
    
    def commands(self):
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
        }
    
    def format_value(self, value):
        try:
            if value.type.code in [gdb.TYPE_CODE_INT, gdb.TYPE_CODE_PTR]:
                int_value = to_unsigned(value, value.type.sizeof)
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
