package korrust.probes.models

interface RecordView {
    val itemCount: Int
    fun itemAt(position: Int): String
    fun label(position: Int): String {
        val next = position + itemCount
        return if (next < 3) {
            LABELS[next]
        } else {
            next.toString()
        }
    }
    interface WithWeight : RecordView {
        fun weight(position: Int): Float
    }
    companion object {
        private val LABELS = arrayOf("zero", "first", "\u0000二")
        val VACANT = object : RecordView {
            override val itemCount: Int
                get() = 7
            override fun itemAt(position: Int): String {
                throw IllegalArgumentException("position=$position count=$itemCount")
            }
        }
    }
}
