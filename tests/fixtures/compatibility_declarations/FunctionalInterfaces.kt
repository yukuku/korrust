package compatibility.declarations

interface MapperParent {
    fun apply(value: Int): Int
}

fun interface Mapper : MapperParent {
    fun twice(value: Int): Int = apply(value) + apply(value)
}

fun interface Sink {
    fun accept(value: String)
}

fun newMapper(amount: Int): Mapper = Mapper { value -> value + amount }

fun capturedMapper(seed: Int): Mapper {
    var total = seed
    return Mapper { value -> total += value; total }
}

fun runSink(sink: Sink) { sink.accept("one"); sink.accept("two") }
fun capturedSink(prefix: String): String {
    var result = ""
    runSink { line -> result += "$prefix:$line;" }
    return result
}

fun select(value: Mapper): String = "sam"
fun select(value: (Int) -> Int): String = "function"
fun distinguishTypes(): String {
    val mapper = Mapper { value -> value + 1 }
    val function: (Int) -> Int = { value -> value + 2 }
    return select(mapper) + ":" + select(function)
}

fun dispatch(value: Mapper): Int { Effects.mark("S"); return value.apply(5) }
fun dispatch(value: (Int) -> Int): Int { Effects.mark("F"); return value(5) }

fun overloadChoices(): String {
    Effects.reset()
    var total = 1
    val direct = dispatch { value -> Effects.mark("L"); total += value; total }
    val sam = dispatch(Mapper { value -> Effects.mark("M"); total += value; total })
    val function: (Int) -> Int = { value -> Effects.mark("T"); total += value; total }
    val typed = dispatch(function)
    return "$direct:$sam:$typed:$total:${Effects.events}"
}
