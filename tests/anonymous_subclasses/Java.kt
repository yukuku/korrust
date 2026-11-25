package anonymousjava
fun javaBase(start: Int): JavaBase {
    var state = start
    return object : JavaBase(start + 1), Runnable {
        override fun compute(value: Int): Int = super.compute(value) + state
        override fun read(): Int = protectedValue() + state
        override fun run() { state += 2 }
    }
}
fun stream(seed: Int): java.io.InputStream = object : java.io.InputStream() {
    override fun read(): Int = seed
}
