#!/usr/bin/perl -w

# Generate usys.S, the stubs for syscalls.

print "# generated by usys.pl - do not edit\n";

print "#include \"kernel/syscall.h\"\n";

$target = $ENV{'TARGET'};

if($target eq "riscv") {
    *entry = sub {
        my $name = shift;
        print ".global $name\n";
        print "${name}:\n";
        print " li a7, SYS_${name}\n";
        print " ecall\n";
        print " ret\n";
    }
}
elsif($target eq "arm") {
    *entry = sub {
        my $name = shift;
        print ".global $name\n";
        print "${name}:\n";
    	print " STR x7, [sp, #-0x08]!\n";
        print " MOV x7, #SYS_${name}\n";
        print " SVC 0x00\n";
        print " LDR x7, [sp], #0x08\n";
        print " br x30;	//lr = x30\n";
    }
}
else {
    exit(1);
}

	
entry("fork");
entry("exit");
entry("wait");
entry("pipe");
entry("read");
entry("write");
entry("close");
entry("kill");
entry("exec");
entry("open");
entry("mknod");
entry("unlink");
entry("fstat");
entry("link");
entry("mkdir");
entry("chdir");
entry("dup");
entry("getpid");
entry("sbrk");
entry("sleep");
entry("uptime");
entry("poweroff");
entry("select");
entry("getpagesize");
entry("waitpid");
entry("getppid");
entry("lseek");
entry("uptime_as_micro");
entry("clock");
