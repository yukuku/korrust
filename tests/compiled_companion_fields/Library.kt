package storage

class Holder(val id: Int) {
    companion object {
        @JvmField val first = Holder(4)
        @JvmField var last: Holder? = null
        internal const val hidden = 13
    }
}
class Named {
    companion object State {
        @JvmField val value: String = "named"
    }
}
