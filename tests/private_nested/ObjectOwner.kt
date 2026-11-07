package privateprobe
object ObjectOwner {
    private class Reader(private val bytes: ByteArray) {
        fun read(): Int = bytes[0].toInt()
    }
    fun read(): Int = Reader(byteArrayOf(31)).read()
}
class Depth {
    private class Hidden {
        class Public(val x: Int)
        fun read(): Int = Public(23).x
    }
    fun read(): Int = Hidden().read()
    private fun create(): Hidden = Hidden()
    fun viaPrivate(): Int = create().read()
}
