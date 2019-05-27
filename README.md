# gdb-dashboard-svdregisters

Tool and module for adding any register to gdbinit from svd-file

#### svdregisters.py is a module for https://github.com/cyrus-and/gdb-dashboard 
It can be used for simple monitoring of Cortex-M registers.

There are several commands.
*bin*, *hex* and *decimal* change Numeral system.

*changed* turns on/off info how value changed at last step

*monitor* and *remove* for adding and removing Register or Field to list.
Requere register name(alias) as argument

*set* command used for changing value of Register by name.
Requere register name and value as arguments

Copy or link it into ~/.gdbinit.d

svdregisters.py reads file "registers.txt" from current directory

#### File format:
* 1 line - reserved for SVD filename.
* other lines - 2 variants:
  * for register:  name, alias("\_" by default), address
  * for field:     name, alias, address, bit_offset, bit_width

*name* is the register path in SVD.

*alias* is register name in dashboard (same as *name* if "\_")

*address* is register address in memory

#### There is also GTK-rs GUI interface for simple choise registers from Cortex-M SVD file.

Compile:
```
cargo build --release
```
and run from your hardware source directory.

SVDs for STM32 can be found [here](https://stm32.agg.io/rs/).