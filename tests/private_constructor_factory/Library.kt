package factory
class Envelope<T> private constructor(val value: T) {
    companion object {
        fun <T> of(value: T): Envelope<T> = Envelope(value)
    }
}
fun <T> Envelope(value: T, count: Int = 1): Envelope<T> {
    require(count == 1)
    return Envelope.of(value)
}
