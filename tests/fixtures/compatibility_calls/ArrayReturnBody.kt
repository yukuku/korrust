package compatibility.calls
import compatibility.calls.lib.*
fun arrayReturnBody(): String {
    CallEffects.reset()
    val input = CallEffects.ints("S")
    val result = arrayWithEffect(*input)
    return "${result[0]}:${input[0]}:${CallEffects.trace()}"
}
