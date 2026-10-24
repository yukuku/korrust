package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun captureProbe(base: Int): Int { val offset = base + 2; return applyValue(3) { value -> value + offset } }
fun mutableCaptureProbe(): Int { var total = 2; val block: (Int) -> Int = { value -> total += value; total }; val alias = block; alias(3); return block(4) }
