package anonymousjava
fun action(values: IntArray): Runnable = object : Runnable {
    override fun run() { values[0] = values[0] + 1 }
}
