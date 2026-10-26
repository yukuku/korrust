import interop.lib.allocationGate
fun boundary(): Int = allocationGate { 3 }
