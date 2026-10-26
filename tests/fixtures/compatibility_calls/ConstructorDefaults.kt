package compatibility.calls
import compatibility.calls.lib.*
fun constructorDefaults(): String {
    CallEffects.reset()
    val box = DefaultBox(last = CallEffects.mark("L", 9))
    val first = CallEffects.trace()
    CallEffects.reset()
    val value = receiver().combine(last = CallEffects.mark("Z", 7))
    return "${box.first}:${box.last}:$first|$value:${CallEffects.trace()}"
}
