package unsignedreview
@JvmInline value class Token(val bits: Int) {
    inline fun apply(noinline step: suspend () -> Int): Int = bits
}
