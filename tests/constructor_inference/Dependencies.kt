package constructors.library

interface Maker<T> { fun get(): T }
private class Made<T>(private val value: T) : Maker<T> { override fun get(): T = value }
fun <T> Maker(value: T): Maker<T> = Made(value)

class Private<T> private constructor(val value: T)
