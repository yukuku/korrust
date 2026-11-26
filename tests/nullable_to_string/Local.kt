package localshadow
fun Any?.toString(): String = "local"
fun local(value: Any?): String = value.toString()
