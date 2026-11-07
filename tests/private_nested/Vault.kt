package privateprobe

class Vault {
    private class Entry(val index: Int, val text: String?)
    private data class Record(val id: Int, val text: String)
    private class Reader(private val bytes: ByteArray) {
        fun read(): Int = bytes[0].toInt()
    }
    fun read(): String {
        val first = Entry(7, "ok")
        val record = Record(first.index, first.text!!)
        val copied = record.copy(id = 9)
        return copied.toString() + ":" + Reader(byteArrayOf(42)).read()
    }
    fun closure(): () -> Int = { Entry(13, null).index }
    class Sibling {
        fun read(): Int = Entry(11, null).index
    }
    companion object {
        fun read(): Int = Entry(17, null).index
    }
}
