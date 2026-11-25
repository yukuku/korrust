interface Base<T> { fun get(): T }
fun make(): Base<Unit> = object : Base<Unit> { override fun get(): Unit {} }
