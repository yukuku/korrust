package compatibility.calls
import compatibility.calls.lib.*
fun compiledDefaults(): String {
    CallEffects.reset()
    val a = defaults(last = CallEffects.mark("L", 9))
    val first = CallEffects.trace()
    CallEffects.reset()
    val b = defaults(first = CallEffects.mark("F", 8))
    val second = CallEffects.trace()
    CallEffects.reset()
    val c = defaults(last = CallEffects.mark("L", 9), first = CallEffects.mark("F", 8))
    val third = CallEffects.trace()
    CallEffects.reset()
    val d = defaults()
    return "$a:$first|$b:$second|$c:$third|$d:${CallEffects.trace()}:${nullableDefault()}:${nullableDefault(value = "ok")}"
}
