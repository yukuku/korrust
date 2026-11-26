package library
inline fun <reified T> classOf(): Class<T> = T::class.java
class Scope {
    inline fun <reified T> same(value: Class<*>): Boolean = value == T::class.java
}
