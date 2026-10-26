package compatibility.declarations

class Components(val value: Int) {
    operator fun component1(): Int { Effects.mark("1"); return value + 1 }
    operator fun component2(): Int { Effects.mark("2"); return value + 2 }
    operator fun component3(): Int { Effects.mark("3"); return value + 3 }
}

fun components(value: Int): Components { Effects.mark("I"); return Components(value) }

fun discardedLocal(): String {
    Effects.reset()
    val (_, middle, _) = components(10)
    return "$middle:${Effects.events}"
}

fun discardedAll(): String {
    Effects.reset()
    val (_, _, _) = components(20)
    return Effects.events
}

fun invokePair(action: (Int, Int) -> Int): Int = action(7, 9)
fun discardedParameters(): Int = invokePair { _, value -> value + 1 }
fun consumeComponents(value: Components, action: (Components) -> Int): Int = action(value)

fun discardedLambda(): String {
    Effects.reset()
    val answer = consumeComponents(components(30)) { (_, _, last) -> last }
    return "$answer:${Effects.events}"
}

fun repeatedDiscardParameters(): Int = invokePair { _, _ -> 11 }

fun discardedLoop(): String {
    Effects.reset()
    var total = 0
    for ((_, _, last) in listOf(components(1), components(2))) { total += last }
    return "$total:${Effects.events}"
}
