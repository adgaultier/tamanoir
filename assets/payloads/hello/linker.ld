ENTRY(_start);

SECTIONS
{
    . = ALIGN(16);
    .text :
    {
        *(.text.prologue)
        *(.text)
        *(.text.*)
        *(.text.msg)

    }
    . = ALIGN(16);
    .data :
    {
        *(.rodata)
        *(.rodata.*)
        *(.data)    
    }
    
    /DISCARD/ :
	{
		*(.interp)
		*(.comment)
		*(.debug_frame)
        *(.ARM.exidx)
        *(.reginfo)
	}

}

