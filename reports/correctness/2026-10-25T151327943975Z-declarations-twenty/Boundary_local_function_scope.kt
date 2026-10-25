fun outer() { fun hidden(): Int = 1 }
fun bad(): Int = hidden()
