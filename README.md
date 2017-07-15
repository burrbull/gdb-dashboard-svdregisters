# gdb-dashboard-svdregisters

Tool and module for adding any register to gdbinit from svd-file

#### svdregisters.py is a module for https://github.com/cyrus-and/gdb-dashboard 
It can be used for simple monitoring of cortex-m registers.

There are several commands.
*bin*, *hex* and *decimal* change Numeral system.
*changed* turns on/off info how value changed at last step

svdregisters.py reads file "registers.txt" from current directory

#### File format:
* 1 line - reserved for SVD filename.
* other lines - 2 variants:
  * for register:  name, alias("\_" by default), address
  * for field:     name, alias, address, bit_offset, bit_width

*name* is the register path in SVD.

*alias* is register name in dashboard (same as *name* if "\_")

*address* is register address in memory

#### For simple choise registers from Cortex-M SVD file GTK GUI interface presents.

svdselector.py - Python variant. It require *cmsis_svd* library.

And fast autonomous variant in Rust language.
But it has one limitation - only one SVD file at a time.
