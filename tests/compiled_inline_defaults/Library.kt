package defaults.library

var order: Int = 0
fun record(value: Int): Int {
    order = order * 10 + value
    return value
}

inline fun accumulate(start: Int = record(1), next: Int = record(start + 1), block: (Int) -> Int): Int = block(start + next)

inline fun guarded(value: Int = record(2), block: () -> Int): Int {
    try {
        return value + block()
    } finally {
        record(9)
    }
}

inline fun <reified T> name(type: Class<*> = T::class.java): String = type.name
inline fun <reified T> defaultBlock(seed: T, block: (T) -> T = { it }): T = block(seed)
inline fun <reified T> reifiedBlock(seed: Int = 2, block: () -> String = { T::class.java.name }): String = seed.toString() + block()

inline fun wide(
    p0: Int = record(1),
    p1: Int = 0,
    p2: Int = 0,
    p3: Int = 0,
    p4: Int = 0,
    p5: Int = 0,
    p6: Int = 0,
    p7: Int = 0,
    p8: Int = 0,
    p9: Int = 0,
    p10: Int = 0,
    p11: Int = 0,
    p12: Int = 0,
    p13: Int = 0,
    p14: Int = 0,
    p15: Int = 0,
    p16: Int = 0,
    p17: Int = 0,
    p18: Int = 0,
    p19: Int = 0,
    p20: Int = 0,
    p21: Int = 0,
    p22: Int = 0,
    p23: Int = 0,
    p24: Int = 0,
    p25: Int = 0,
    p26: Int = 0,
    p27: Int = 0,
    p28: Int = 0,
    p29: Int = 0,
    p30: Int = 0,
    p31: Int = record(2),
    p32: Int = record(3),
    block: (Int) -> Int,
): Int = block(p0 + p31 + p32)
