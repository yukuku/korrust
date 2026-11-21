package staticprobe
fun read(): String = Host.first + Global.value
fun qualified(): String = Host.Companion.first
fun update() { Host.mutable = "X"; Global.number = 9 }
