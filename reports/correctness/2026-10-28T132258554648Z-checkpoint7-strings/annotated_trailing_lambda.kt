fun invokeInt(action: () -> Int): Int = action()
fun boundary(): Int = invokeInt()
@Suppress("unused") { 3 }
