import interop.lib.applyValue
fun bad(): Int = applyValue(1) { "wrong" }
