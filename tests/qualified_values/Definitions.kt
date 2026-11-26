package qv.kotlin

object Singleton {
    var count: Int = 2
    fun add(value: Int): Int { count += value; return count }
}

class Host {
    companion object {
        var count: Int = 3
        fun add(value: Int): Int { count += value; return count }
    }
}

val root: qv.p.Root = qv.p.Root()
