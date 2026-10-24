import interop.lib.applyValue
fun bad(): Int { applyValue(1) { return 2 }; return 3 }
