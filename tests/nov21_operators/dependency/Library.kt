package foreign
class Meter(val value: Int) {
    operator fun inc(): Meter = Meter(value + 1)
    operator fun plus(delta: Int): Meter = Meter(value + delta)
}
class Switch(val value: Int)
operator fun Switch.inc(): Switch = Switch(value + 1)
operator fun Switch.dec(): Switch = Switch(value - 1)
class Bucket(var value: Meter) {
    operator fun get(index: Long): Meter = value
    operator fun set(index: Long, next: Meter) { value = next }
}
