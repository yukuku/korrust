package declarations.probes
interface BodyContract {
    val value: Int
    fun calculate(): Int
    fun weighted(text: String, scale: Float): Float = text.length.toFloat() * scale
}
class BodyClass(override val value: Int): BodyContract {
    override fun calculate(): Int { var total = 0; for(index in 0..value) { if(index > 0) total += index }; val plus: (Int) -> Int = { it + 1 }; return plus(total) }
    val doubled = value * 2
    fun inferred() = value + 1
}
object BodyObject: BodyContract {
    override val value = 7
    override fun calculate(): Int { var total = 0; while(total < value) { total += 1 }; val plus: (Int) -> Int = { it + 2 }; return plus(total) }
}
fun bodies(): Int = BodyClass(3).calculate() + BodyObject.calculate()
class PrivateCapture(private var value: Int) {
    fun calc(): Int { val next: () -> Int = { value += 1; value }; return next() }
    private fun secret(): Int = value
    fun invokePrivate(): Int { val read: () -> Int = { secret() }; return read() }
}
