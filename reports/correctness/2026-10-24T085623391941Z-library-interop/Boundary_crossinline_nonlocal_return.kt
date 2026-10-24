import interop.lib.crossinlineGate
fun bad(): Int { crossinlineGate { return 2 }; return 3 }
