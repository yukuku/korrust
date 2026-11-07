package anonymousinline
interface Value { fun read(): Int }
fun inferred(): Value = object : Value {
    override fun read() = run {
        var state = 2
        val increment: () -> Int = { state++; state }
        increment() + state
    }
}
fun conditional(flag: Boolean): Value = object : Value {
    override fun read(): Int {
        run { if (flag) return 7; 3 }
        return 9
    }
}
