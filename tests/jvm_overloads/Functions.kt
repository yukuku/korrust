package overloads

@JvmOverloads
fun top(a: Int = 1, b: String, c: Long = 3L): String = "$a:$b:$c"

class Member {
    @JvmOverloads
    fun value(a: Int = 1, b: String = "member"): String = "$a:$b"

    companion object {
        @JvmStatic
        @JvmOverloads
        fun staticValue(a: Int = 2, b: String = "companion"): String = "$a:$b"

        @JvmOverloads
        fun instance(a: Long = 4L): Long = a
    }
}

object Singleton {
    @JvmStatic
    @JvmOverloads
    fun staticValue(a: Int = 5, b: String = "object"): String = "$a:$b"

    @JvmOverloads
    fun instance(a: Long = 6L): Long = a
}

@JvmOverloads
fun <T> generic(value: T, ignored: Int = 7): T = value

@JvmOverloads
fun nullable(value: String? = null, amount: Double = 2.5): Double = if (value == null) amount else amount + 1.0

@JvmOverloads
fun effects(a: Int = Effects.mark(1), required: String, c: Int = Effects.mark(a + 2)): String = "$a:$required:$c"

@JvmOverloads
fun throwing(a: Int = Effects.fail(), required: Int): Int = a + required

@JvmOverloads
fun varargs(prefix: String = "V", vararg values: Int): String = "$prefix:${values.size}"

@JvmOverloads
fun mask32(p0: Int = 1, p1: Int, p2: Int, p3: Int, p4: Int, p5: Int, p6: Int, p7: Int, p8: Int, p9: Int, p10: Int, p11: Int, p12: Int, p13: Int, p14: Int, p15: Int, p16: Int, p17: Int, p18: Int, p19: Int, p20: Int, p21: Int, p22: Int, p23: Int, p24: Int, p25: Int, p26: Int, p27: Int, p28: Int, p29: Int, p30: Int, p31: Int = 32): Int = p0 + p15 + p31

@JvmOverloads
fun unchanged(value: Int): Int = value

@Marker("method")
@JvmOverloads
fun annotated(@Marker("first") first: Int = 1, @Marker("required") required: String, @Marker("last") last: Long = 2L): String = "$first:$required:$last"

class Annotations {
    companion object {
        @JvmStatic
        @JvmOverloads
        @Marker("companion")
        fun marked(@Marker("required") value: String, amount: Int = 1): String = value
    }
}

object Exceptional {
    @JvmStatic
    @JvmOverloads
    @Throws(IllegalStateException::class)
    fun value(a: Int = 7): Int = a
}

class Stateful(private val seed: Int) {
    @JvmOverloads
    fun value(amount: Int = seed): Int = amount
}

object DefaultState {
    private val seed: Int = 17

    @JvmOverloads
    fun value(amount: Int = seed): Int = amount
}

class CompanionState {
    companion object {
        private val seed: Int = 19

        @JvmOverloads
        fun value(amount: Int = seed): Int = amount
    }
}

open class VirtualBase {
    @JvmOverloads
    open fun value(amount: Int = 29): Int = amount
}

class VirtualDerived : VirtualBase() {
    override fun value(amount: Int): Int = amount + 10
}
