fun position(values: Array<String>, key: String): Int = values.indexOf(key)
fun nullable(values: Array<String?>): Int = values.indexOf(null)
fun projected(values: Array<out Number>, key: Int): Int = values.indexOf(key)
fun fromJava(): Int = Platform.labels().indexOf("ok")
