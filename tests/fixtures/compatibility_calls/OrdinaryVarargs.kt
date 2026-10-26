package compatibility.calls
import compatibility.calls.lib.*
fun ordinaryVarargs(): String {
    CallEffects.reset()
    val empty = CallEffects.sum()
    val single = CallEffects.sum(4)
    val multiple = CallEffects.sum(CallEffects.mark("A", 2), CallEffects.mark("B", 3))
    val kotlin = kotlinInts() + kotlinInts(7) + kotlinInts(1, 2, 3)
    val refs = kotlinStrings(null, "x") + genericCount<Int>(1, 2) + CallEffects.count(1, null, "s") + genericText("a", "b")
    val primitiveComponent = component<Int>(1, 2)
    val referenceComponent = component("a", "b")
    return "$empty:$single:$multiple:$kotlin:$refs:${CallEffects.choose(4)}:${CallEffects.choose(1, 2)}:${CallEffects.trace()}:$primitiveComponent:$referenceComponent"
}
