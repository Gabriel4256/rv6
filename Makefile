K = kernel
U = user
KR = kernel-rs
LM = lmbench

RUST_TARGET = aarch64-unknown-none
ifndef RUST_MODE
RUST_MODE = debug
endif

ifeq ($(RUST_MODE),release)
CARGOFLAGS = --release
else
CARGOFLAGS =
endif

# OBJS = \
#   $K/entry.o \
#   $K/start.o \
#   $K/console.o \
#   $K/printf.o \
#   $K/uart.o \
#   $K/kalloc.o \
#   $K/spinlock.o \
#   $K/string.o \
#   $K/main.o \
#   $K/vm.o \
#   $K/proc.o \
#   $K/swtch.o \
#   $K/trampoline.o \
#   $K/trap.o \
#   $K/syscall.o \
#   $K/sysproc.o \
#   $K/bio.o \
#   $K/fs.o \
#   $K/log.o \
#   $K/sleeplock.o \
#   $K/file.o \
#   $K/pipe.o \
#   $K/exec.o \
#   $K/sysfile.o \
#   $K/kernelvec.o \
#   $K/plic.o \
#   $K/virtio_disk.o \
#   $(KR)/target/$(RUST_TARGET)/$(RUST_MODE)/librv6_kernel.a

OBJS = \
  $K/entry.o \
  $K/swtch.o \
  $K/trampoline.o \
  $K/kernelvec.o \
  $(KR)/target/$(RUST_TARGET)/$(RUST_MODE)/librv6_kernel.a

# riscv64-unknown-elf- or riscv64-linux-gnu-
# perhaps in /opt/riscv/bin
TOOLPREFIX = aarch64-none-elf-

# Try to infer the correct TOOLPREFIX if not set
ifndef TOOLPREFIX
TOOLPREFIX := $(shell if aarch64-unknown-elf-objdump -i 2>&1 | grep 'elf64-big' >/dev/null 2>&1; \
	then echo 'aarch64-unknown-elf-'; \
	elif aarch64-linux-gnu-objdump -i 2>&1 | grep 'elf64-big' >/dev/null 2>&1; \
	then echo 'aarch64-linux-gnu-'; \
	elif aarch64-unknown-linux-gnu-objdump -i 2>&1 | grep 'elf64-big' >/dev/null 2>&1; \
	then echo 'aarch64-unknown-linux-gnu-'; \
	else echo "***" 1>&2; \
	echo "*** Error: Couldn't find a aarch64 version of GCC/binutils." 1>&2; \
	echo "*** To turn off this error, run 'gmake TOOLPREFIX= ...'." 1>&2; \
	echo "***" 1>&2; exit 1; fi)
endif

QEMU = qemu-system-aarch64

CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gas
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump

ifndef OPTFLAGS
OPTFALGS := -O
endif

CFLAGS = -Wall -Werror $(OPTFLAGS) -fno-omit-frame-pointer -ggdb
CFLAGS += -MD
CFLAGS += -ffreestanding -fno-common -nostdlib
CFLAGS += -I.
CFLAGS += $(shell $(CC) -fno-stack-protector -E -x c /dev/null >/dev/null 2>&1 && echo -fno-stack-protector)

ifeq ($(USERTEST),yes)
CFLAGS += -DUSERTEST
endif

# Disable PIE when possible (for Ubuntu 16.10 toolchain)
ifneq ($(shell $(CC) -dumpspecs 2>/dev/null | grep -e '[^f]no-pie'),)
CFLAGS += -fno-pie -no-pie
endif
ifneq ($(shell $(CC) -dumpspecs 2>/dev/null | grep -e '[^f]nopie'),)
CFLAGS += -fno-pie -nopie
endif

LDFLAGS = -z max-page-size=4096

$K/kernel: $(OBJS) $K/kernel.ld $U/initcode $U/initcode fs.img 
	$(LD) $(LDFLAGS) -T $K/kernel.ld -o $K/kernel $(OBJS) -b binary $U/initcode fs.img 
	$(OBJDUMP) -S $K/kernel > $K/kernel.asm
	$(OBJDUMP) -t $K/kernel | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $K/kernel.sym

$U/initcode: $U/initcode.S
	$(CC) $(CFLAGS) -march=armv8-a -mcpu=cortex-a57 -nostdinc -I. -Ikernel -c $U/initcode.S -o $U/initcode.o
	$(LD) $(LDFLAGS) -N -e start -Ttext 0 -o $U/initcode.out $U/initcode.o
	$(OBJCOPY) -S -O binary $U/initcode.out $U/initcode
	$(OBJDUMP) -S $U/initcode.o > $U/initcode.asm

$(KR)/target/$(RUST_TARGET)/$(RUST_MODE)/librv6_kernel.a: $(shell find $(KR) -type f)
	cargo build --manifest-path kernel-rs/Cargo.toml --target kernel-rs/$(RUST_TARGET).json $(CARGOFLAGS)

tags: $(OBJS) _init
	etags *.S *.c

ULIB = $U/ulib.o $U/usys.o $U/printf.o $U/umalloc.o $U/string.o

_%: %.o $(ULIB) $U/rand.o
	$(LD) $(LDFLAGS) -N -e main -Ttext 0 -o $@ $^
	$(OBJDUMP) -S $@ > $*.asm
	$(OBJDUMP) -t $@ | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $*.sym

$U/usys.S : $U/usys.pl
	perl $U/usys.pl > $U/usys.S

$U/usys.o : $U/usys.S
	$(CC) $(CFLAGS) -c -o $U/usys.o $U/usys.S

$U/_forktest: $U/forktest.o $(ULIB)
	# forktest has less library code linked in - needs to be small
	# in order to be able to max out the proc table.
	$(LD) $(LDFLAGS) -N -e main -Ttext 0 -o $U/_forktest $U/forktest.o $U/ulib.o $U/usys.o
	$(OBJDUMP) -S $U/_forktest > $U/forktest.asm

$U/_usertests: $U/usertests.o $(ULIB)
	$(LD) $(LDFLAGS) -N -e main -Ttext 0 -o $@ $^
	$(OBJDUMP) -S $@ > $*.asm
	$(OBJDUMP) -t $@ | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $*.sym

$U/_grind: $U/grind.o $(ULIB)
	$(LD) $(LDFLAGS) -N -e main -Ttext 0 -o $@ $^
	$(OBJDUMP) -S $@ > $*.asm
	$(OBJDUMP) -t $@ | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $*.sym

## LMbench
$(LM)/%.o: $(LM)/%.c
	$(CC) $(CFLAGS) -c -o $@ $^


$U/_%: $(LM)/%.o $(ULIB) $(LM)/lmbench.a $U/rand.o
	$(LD) $(LDFLAGS) -N -e main -Ttext 0 -o $@ $^
	$(OBJDUMP) -S $@ > $*.asm
	$(OBJDUMP) -t $@ | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $*.sym

AR=ar
ARCREATE=cr
LIBOBJS= $(LM)/lib_timing.o 	\
	$(LM)/lib_mem.o $(LM)/lib_stats.o $(LM)/lib_debug.o $(LM)/getopt.o		\
	$(LM)/lib_sched.o
INCS = $(LM)/bench.h $(LM)/lib_mem.h $(LM)/lib_tcp.h $(LM)/lib_udp.h $(LM)/stats.h $(LM)/timing.h

$(LM)/lmbench : ../scripts/lmbench version.h
	rm -f $(LM)/lmbench
	VERSION=`../scripts/version`; \
	sed -e "s/<version>/$${VERSION}/g" < ../scripts/lmbench > $(LM)/lmbench
	chmod +x $(LM)/lmbench

$(LM)/lmbench.a: $(LIBOBJS)
	/bin/rm -f $(LM)/lmbench.a
	$(AR) $(ARCREATE) $(LM)/lmbench.a $(LIBOBJS)
	-ranlib $(LM)/lmbench.a

$(LM)/lib_timing.o : $(LM)/lib_timing.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_timing.c -o $(LM)/lib_timing.o
$(LM)/lib_mem.o : $(LM)/lib_mem.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_mem.c -o $(LM)/lib_mem.o
$(LM)/lib_tcp.o : $(LM)/lib_tcp.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_tcp.c -o $(LM)/lib_tcp.o
$(LM)/lib_udp.o : $(LM)/lib_udp.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_udp.c -o $(LM)/lib_udp.o
$(LM)/lib_unix.o : $(LM)/lib_unix.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_unix.c -o $(LM)/lib_unix.o
$(LM)/lib_debug.o : $(LM)/lib_debug.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_debug.c -o $(LM)/lib_debug.o
$(LM)/lib_stats.o : $(LM)/lib_stats.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_stats.c -o $(LM)/lib_stats.o
$(LM)/lib_sched.o : $(LM)/lib_sched.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/lib_sched.c -o $(LM)/lib_sched.o
$(LM)/getopt.o : $(LM)/getopt.c $(INCS)
	$(CC) $(CFLAGS) -c $(LM)/getopt.c -o $(LM)/getopt.o


mkfs/mkfs: mkfs/mkfs.c $K/fs.h $K/param.h
	gcc -Werror -Wall -I. -o mkfs/mkfs mkfs/mkfs.c

# Prevent deletion of intermediate files, e.g. cat.o, after first build, so
# that disk image changes after first build are persistent until clean.  More
# details:
# http://www.gnu.org/software/make/manual/html_node/Chained-Rules.html
.PRECIOUS: %.o

UPROGS=\
	$U/_cat\
	$U/_echo\
	$U/_forktest\
	$U/_grep\
	$U/_init\
	$U/_kill\
	$U/_ln\
	$U/_ls\
	$U/_mkdir\
	$U/_rm\
	$U/_sh\
	$U/_stressfs\
	$U/_usertests\
	$U/_grind\
	$U/_wc\
	$U/_zombie\
	# $U/_msleep\
	$U/_bw_file_rd\
	$U/_bw_mem\
	$U/_bw_pipe\
	$U/_lat_ctx\
	$U/_lat_fifo\
	$U/_lat_fs\
	# $U/_lat_mem_rd\
	# $U/_lat_ops\
	# $U/_lat_pipe\
	# $U/_lat_proc\
	# $U/_lat_syscall\
	# $U/_mhz\
	# $U/_tlb\
	# $U/_line\
	# $U/_cache\
	# $U/_stream\
	# $U/_par_mem\
	# $U/_par_ops\

fs.img: mkfs/mkfs README $(UPROGS)
	mkfs/mkfs fs.img README $(UPROGS)

-include kernel/*.d user/*.d

clean: 
	rm -f *.tex *.dvi *.idx *.aux *.log *.ind *.ilg \
	*/*.o */*.d */*.asm */*.sym \
	$(KR)/target/$(RUST_TARGET)/$(RUST_MODE)/librv6_kernel.a \
	$U/initcode $U/initcode.out $K/kernel fs.img \
	mkfs/mkfs .gdbinit \
        $U/usys.S \
	$(UPROGS) \
	$(LM)/*.[oasd]
	cargo clean --manifest-path $(KR)/Cargo.toml

# try to generate a unique GDB port
GDBPORT = $(shell expr `id -u` % 5000 + 25000)
# QEMU's gdb stub command line changed in 0.11
QEMUGDB = $(shell if $(QEMU) -help | grep -q '^-gdb'; \
	then echo "-gdb tcp::$(GDBPORT)"; \
	else echo "-s -p $(GDBPORT)"; fi)
ifndef CPUS
CPUS := 3
endif

QEMUOPTS = -machine virt -cpu cortex-a57 -kernel $K/kernel -m 128M -smp $(CPUS) -nographic
QEMUOPTS += -drive file=fs.img,if=none,format=raw,id=x0
QEMUOPTS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

qemu: $K/kernel fs.img
	$(QEMU) $(QEMUOPTS)

.gdbinit: .gdbinit.tmpl-aarch64
	sed "s/:1234/:$(GDBPORT)/" < $^ > $@

qemu-gdb: $K/kernel .gdbinit fs.img
	@echo "*** Now run 'gdb' in another window." 1>&2
	$(QEMU) $(QEMUOPTS) -S $(QEMUGDB)

doc: $(KR)/src $(KR)/Cargo.lock $(KR)/Cargo.toml $(KR)/aarch64-unknown-none-elfhf.json
	cargo rustdoc --manifest-path kernel-rs/Cargo.toml -- --document-private-items -A non_autolinks
