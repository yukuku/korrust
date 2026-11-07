package anonymousjava
fun run(base: Int): java.util.concurrent.Callable<String> = object : java.util.concurrent.Callable<String> {
    override fun call(): String = "value" + base
}
