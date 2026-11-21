package datarefs

fun defaultCopy(value: Selection): Selection = value.copy()
fun emptyCopy(value: Selection): Selection = value.copy(ids = emptySet())
fun first(value: Selection): Set<Int> = value.component1()
fun option(value: Selection): Set<Int>? = value.optional
fun create(ids: Set<Int>, names: Map<String, String?>, boxed: Crate<String>): Selection = Selection(ids, names, boxed)
