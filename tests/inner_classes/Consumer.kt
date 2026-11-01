package innerconsumer
import innerprobe.Outer
import innerprobe.GenericArgumentsOuter
fun consume(): Int {
    val entry = GenericArgumentsOuter().Entry(listOf("OK"))
    if (entry.values[0] != "OK") throw IllegalStateException("generic constructor")
    val first = Outer(9)
    val inner = first.Inner(4)
    return inner.read() + inner.outer().value + inner.delta
}
