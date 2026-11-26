package lib
fun fail(message: String): Nothing = throw IllegalArgumentException(message)
fun absent(): Nothing? = null
fun javaVoid(): java.lang.Void? = null
