package typevars

open class Base<X>(val seed: X) {
    open fun apply(value: X): X = value
}
interface Mapper<X> {
    fun map(value: X): X
}
fun <T> fromFunction(value: T): Base<T> = object : Base<T>(value) {
    override fun apply(argument: T): T = value
}
fun <T> fromNested(value: List<T>): Mapper<List<T>> = object : Mapper<List<T>> {
    override fun map(argument: List<T>): List<T> = value
}
class Holder<T>(val value: T) {
    fun fromClass(): Base<T> = object : Base<T>(value) {
        override fun apply(argument: T): T = value
    }
    fun <U> fromMember(argument: U): Mapper<U> = object : Mapper<U> {
        override fun map(other: U): U = argument
    }
    val initialized: Mapper<T> = object : Mapper<T> {
        override fun map(other: T): T = value
    }
}
fun <T : Number> bounded(value: T): Base<T> = object : Base<T>(value) {
    override fun apply(argument: T): T = value
}

interface Factory<T> {
    fun make(value: T): Mapper<T> = object : Mapper<T> {
        override fun map(argument: T): T = value
    }
}
class FactoryImpl : Factory<String>
