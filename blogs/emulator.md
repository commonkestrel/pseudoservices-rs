This is my process of making an emulator for my custom CPU architecture.

In the previous article, we discussed creating an assembler,
which would take input like this:

```avrasm
LW r0, 100

.loop:
    DEC r0
    JNZ r0, .loop
```

It would then assemble it into machine code readable by the CPU:

`0x78 0x0A 0x18 0x01 0xD0 0x00 0x02`

## Why an emulator?

But what do we do with that machine code?
Since we don't have a physical CPU yet,
this is where the emulator comes in.

An emulator will take the machine code and run it as if it were on the actual CPU.
One benifit of the emulator, however, is that we can control exactly what is happening,
which is useful for testing. For example, if we need to know what's in a register at a certain time,
we can simply pause the program and check.
