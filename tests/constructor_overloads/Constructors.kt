package constructors

import kotlin.jvm.JvmOverloads
import kotlin.jvm.JvmOverloads as Overloads

@kotlin.Suppress("unused")
class Interleaved @JvmOverloads constructor(
    val a: Int = Effects.mark(1),
    @Mark("required") val required: String,
    val b: Long = Effects.mark(2).toLong(),
    val tail: Int,
    @Mark("last") val c: Int = Effects.mark(3),
) {
    init { Effects.mark(4) }
}

class Dependent @Overloads public constructor(
    val a: Int = Effects.mark(5),
    val b: Int = a + Effects.mark(6),
)

class Exceptional @kotlin.jvm.JvmOverloads constructor(
    val required: String,
    val first: Int = Effects.mark(7),
    val failure: Int = Effects.fail(),
    val last: Int = Effects.mark(8),
)

class Decorated @Mark("con" + "structor") @JvmOverloads constructor(
    @Mark("text") val text: String,
    @Mark("number") val number: Int = 9,
)

open class Parent(val value: Int)
class Child @JvmOverloads constructor(value: Int = Effects.mark(9)) : Parent(value)

data class Record @JvmOverloads constructor(val label: String, val count: Int = 2)
internal class Internal @JvmOverloads constructor(val value: Int = 10)
class NoDefaults @JvmOverloads constructor(val value: Int)
class Explicit public constructor(val value: Int)

class Mask32 @JvmOverloads constructor(
    p0: Int = 0,
    p1: Int = 1,
    p2: Int = 2,
    p3: Int = 3,
    p4: Int = 4,
    p5: Int = 5,
    p6: Int = 6,
    p7: Int = 7,
    p8: Int = 8,
    p9: Int = 9,
    p10: Int = 10,
    p11: Int = 11,
    p12: Int = 12,
    p13: Int = 13,
    p14: Int = 14,
    p15: Int = 15,
    p16: Int = 16,
    p17: Int = 17,
    p18: Int = 18,
    p19: Int = 19,
    p20: Int = 20,
    p21: Int = 21,
    p22: Int = 22,
    p23: Int = 23,
    p24: Int = 24,
    p25: Int = 25,
    p26: Int = 26,
    p27: Int = 27,
    p28: Int = 28,
    p29: Int = 29,
    p30: Int = 30,
    val p31: Int = Effects.mark(1)
)

class Wide @JvmOverloads constructor(
    val required: Long,
    val first: Double = 1.5,
    val second: Float = 2.5f,
    val tail: String,
)
class Nullable @JvmOverloads constructor(val required: Int, val text: String? = null)

class Automatic @Mark("auto") constructor(val value: Int = 13)
