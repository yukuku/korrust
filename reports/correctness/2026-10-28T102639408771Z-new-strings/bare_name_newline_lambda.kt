fun invokeInt(action: () -> Int): Int = action()
fun bad(): Int = invokeInt
{ 3 }
