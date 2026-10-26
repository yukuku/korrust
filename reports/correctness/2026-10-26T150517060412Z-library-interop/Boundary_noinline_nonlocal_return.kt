import interop.lib.noinlineGate
fun bad(): Int { noinlineGate { return 2 }; return 3 }
