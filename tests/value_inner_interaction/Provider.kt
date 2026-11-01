package valueinner

@JvmInline
value class Counter(val raw: Int)

class Outer(var base: Int) {
    inner class Reader(val increment: Int) {
        fun read(): Int = Counter(base + increment).raw
        fun update(value: Int): Int {
            base = Counter(value).raw
            return read()
        }
    }
}

enum class Unrelated { FIRST, SECOND }

fun count(): Int = Counter(Unrelated.values().size).raw

fun identityChecks(): Boolean {
    val absent: Counter? = null
    val present: Counter? = Counter(1)
    return absent === null && null === absent && present !== null && null !== present
}
