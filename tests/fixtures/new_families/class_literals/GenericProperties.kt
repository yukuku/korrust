package newfamilies.dependency

import kotlin.reflect.KClass

val <T : Any> KClass<T>.relay: Class<T>
    get() = this.java
val <T : Any> KClass<T>.nested: List<Class<T>>
    get() = listOf(this.java)
fun <T : Any> KClass<T>.consume(value: T): T = value
fun <T> identity(value: T): T = value
val <T : Any> List<T>.head: T
    get() = this[0]
