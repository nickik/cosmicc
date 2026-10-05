/* Minimal ext2 overwrite: 1 KiB blocks, 128-byte inodes, one block group,
 * one preallocated 32-byte /note.txt. No allocator, libc, or 64-bit arithmetic.
 * The guest discovers disk addresses from ext2 metadata, not host arguments.
 * QDX-B namespace 2 uses the established Pico queue and PLIO DMA protocol.
 */
#define U32(a) (*(volatile unsigned *)(a))
#define U16(a) (*(volatile unsigned short *)(a))
#define U8(a) (*(volatile unsigned char *)(a))
#define HOST 0xffe00000U
#define WORKER 0xffe80000U

unsigned minimal_write(void) {
    unsigned tail=0U, table=0U, inode=0U, block=0U, off, budget, rec, name;
    unsigned state=0U, op=0x10U, lba=1U;
    unsigned *sq;
    U32(HOST+0x100U)=0x80000000U;
    U32(HOST+0x1000U)=0U;
    U32(HOST+0x1004U)=0x100000U;
    U32(HOST+0x1008U)=7U;
    U32(WORKER+0x1010U)=0x5000U; U16(WORKER+0x1014U)=4U;
    U32(WORKER+0x1020U)=0x6000U; U16(WORKER+0x1024U)=4U;
    U32(WORKER+0x1008U)=1U;
    while (1) {
        sq=(unsigned *)(0x5000U+(tail&3U)*32U);
        sq[0]=0x20000U|op; sq[1]=tail+1U; sq[2]=lba;
        sq[3]=op==0x12U?0U:1U; sq[4]=op==0x12U?0U:0x7000U;
        sq[5]=0U; sq[6]=0U; sq[7]=0U;
        U32(0x600cU+(tail&3U)*16U)=0xffffffffU;
        U16(WORKER+0x1018U)=tail+1U;
        budget=1000U;
        while (U32(0x600cU+(tail&3U)*16U)==0xffffffffU) {
            budget=budget-1U; if (budget==0U) return 10U;
        }
        if (U16(0x6004U+(tail&3U)*16U)!=0U) return 11U;
        /* Drain the local QLI completion tail before touching worker MMIO. */
        for (off=0U; off<32U; off=off+1U) { budget=budget+1U; }
        tail=tail+1U; U16(WORKER+0x1028U)=tail;
        switch (state) {
        case 0:
            if (U16(0x7038U)!=0xef53U || U32(0x7018U)!=0U ||
                U16(0x7058U)!=128U || U32(0x7060U)!=0U ||
                U32(0x7004U)>U32(0x7020U)) return 20U;
            lba=2U; break;
        case 1:
            table=U32(0x7008U); lba=table; break;
        case 2:
            if ((U16(0x7080U)&0xf000U)!=0x4000U) return 21U;
            lba=U32(0x70a8U); break;
        case 3:
            /* Fixture directory entries start in its first 512-byte sector. */
            off=0U;
            while (off<512U) {
                rec=U16(0x7004U+off); name=U16(0x7006U+off);
                if (rec<8U || (rec&3U)!=0U || off+rec>1024U ||
                    name>rec-8U || off+8U+name>512U) return 22U;
                if (name==8U && U32(0x7008U+off)==0x65746f6eU &&
                    U32(0x700cU+off)==0x7478742eU) { inode=U32(0x7000U+off); break; }
                off=off+rec;
            }
            if (inode==0U) return 23U;
            lba=table+((inode-1U)>>3); break;
        case 4:
            off=((inode-1U)&7U)*128U;
            if ((U16(0x7000U+off)&0xf000U)!=0x8000U ||
                U32(0x7004U+off)!=32U || U32(0x701cU+off)!=2U) return 24U;
            block=U32(0x7028U+off);
            if (block==0U) return 25U;
            lba=block; break;
        case 5:
            for (off=0U; off<32U; off=off+1U) if (U8(0x7000U+off)!=65U) return 26U;
            /* Exactly 32 bytes: "Hello from Lighting via QDX-B!!\n". */
            U32(0x7000U)=0x6c6c6548U; U32(0x7004U)=0x7266206fU;
            U32(0x7008U)=0x4c206d6fU; U32(0x700cU)=0x74686769U;
            U32(0x7010U)=0x20676e69U; U32(0x7014U)=0x20616976U;
            U32(0x7018U)=0x2d584451U; U32(0x701cU)=0x0a212142U;
            op=0x11U; break;
        case 6: op=0x12U; lba=0U; break;
        case 7: op=0x10U; lba=block; break;
        case 8:
            if (U32(0x7000U)!=0x6c6c6548U || U32(0x701cU)!=0x0a212142U) return 27U;
            return 0U;
        }
        state=state+1U;
    }
    return 99U;
}
