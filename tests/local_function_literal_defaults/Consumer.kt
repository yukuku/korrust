import lib.read
fun check(): Int {
    var calls = 0
    fun tick(): Int { calls += 1; return calls }
    fun sum(a: Int = 2, b: Int = 3): Int = a * 10 + b
    val nested = read { sum(b = tick(), a = tick()) }
    val shadow = read {
        fun sum(a: Int = 4): Int = a
        sum()
    }
    return nested * 100 + sum() + shadow
}
