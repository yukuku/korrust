package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun inlineProbe(): Int = actualInline(JavaApi.mark("A", 5)) { value -> JavaApi.mark("B", value * 2) }
fun inlineCaptureProbe(value: Int): Int { val captured = value + 3; return twice { captured } }
fun inlineMutableCaptureProbe(): Int { var value = 0; val result = twice { value++; value }; return result * 10 + value }
fun inlineBranchProbe(flag: Boolean): Int = branch(flag) { it * 4 }
fun inlineSimpleProbe(): Int = inlineSimple(9)
