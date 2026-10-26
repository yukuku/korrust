import declarations.library.invokeInt
fun bad() { while(true) { invokeInt { continue; 1 } } }
