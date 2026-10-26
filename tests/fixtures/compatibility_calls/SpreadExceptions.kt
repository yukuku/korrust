package compatibility.calls
import compatibility.calls.lib.*
fun spreadExceptions(): String {
    CallEffects.reset()
    try { kotlinInts(CallEffects.mark("A", 1), *CallEffects.ints("B"), CallEffects.fail(), CallEffects.mark("C", 4)) }
    catch (error: IllegalStateException) { return CallEffects.trace() }
    return "missing exception"
}
