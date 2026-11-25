interface Mapper<X> { fun map(value: X): X }
class Holder<T> {
    fun <T> make(value: T): Mapper<T> = object : Mapper<T> {
        override fun map(argument: T): T = value
    }
}
