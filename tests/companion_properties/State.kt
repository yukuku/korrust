package companionstate

class Store(seed: Int) {
    init {
        hidden = seed
        custom = seed
        raw = "init"
    }
    fun assign(next: Int): Int {
        hidden = next
        custom = next
        raw = "assign"
        return hidden + custom
    }
    fun delayed(next: Int): () -> Int = {
        hidden = next
        custom = next
        raw = "lambda"
        hidden + custom
    }
    fun local(initial: Int): Int {
        var custom = initial
        custom = custom + 1
        return custom
    }
    fun read(): Int = hidden + custom
    companion object {
        private var hidden: Int = 0
        var custom: Int = 0
            set(value) { field = value * 2 }
        @JvmField var raw: String? = null
        val fixed: Int = 9
    }
}

class Shadow {
    var count: String = "instance"
    fun change(next: String): String {
        count = next
        return count
    }
    companion object { var count: Int = 70 }
}

interface Labels {
    fun first(): String = WORDS[0]
    companion object { private val WORDS = arrayOf("interface-label") }
}
class LabelUser : Labels
