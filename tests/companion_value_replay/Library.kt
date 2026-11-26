package companionvalues.library
@JvmInline value class Key(val raw: Long) {
    companion object { val zero: Key = Key(0L) }
}
@JvmInline value class Label(val raw: String)
@JvmInline value class OtherKey(val raw: Long)
class Palette {
    companion object {
        val primary: Key = Key(47L)
        val name: Label = Label("palette")
        var accent: Key = Key(5L)
        val maybe: Key? = null
        const val version: Int = 3
        @JvmField val javaName: String = "direct"
        private val secret: Key = Key(91L)
        fun choose(key: Key = Key(71L)): Key = key
    }
}
