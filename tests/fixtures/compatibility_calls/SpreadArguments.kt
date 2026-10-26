package compatibility.calls
import compatibility.calls.lib.*
fun spreadArguments(): String {
    CallEffects.reset()
    val values = CallEffects.ints("S")
    val original = CallEffects.mutate(*values)
    val copied = preserve(*values)
    copied[0] = 88
    val named = preserve(values = values)
    named[0] = 77
    val sum = kotlinInts(CallEffects.mark("A", 1), *CallEffects.ints("B"), CallEffects.mark("C", 4), *IntArray(0))
    val strings = CallEffects.strings("T")
    val old = CallEffects.mutateStrings(*strings)
    return "$original:${values[0]}:$sum:$old:${strings[0]}:${CallEffects.trace()}"
}
