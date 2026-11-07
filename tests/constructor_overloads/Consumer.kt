package constructors
fun consume(): String {
    Effects.reset()
    val value = Interleaved(tail = 8, required = "K")
    val dependent = Dependent(b = 20)
    val record = Record(label = "R")
    val custom = custom.Lookalike(required = 4)
    if (value.a != 1 || value.b != 2L || value.c != 3 || value.tail != 8) return "interleaved"
    if (dependent.a != 5 || dependent.b != 20 || record.count != 2 || custom.optional != 7) return "defaults"
    if (Effects.log != "12345") return Effects.log
    val constructor = Decorated::class.constructors.single()
    if (constructor.annotations.filterIsInstance<Mark>().single().value != "constructor") return "constructor annotation"
    if (constructor.parameters[0].annotations.filterIsInstance<Mark>().single().value != "text") return "parameter annotation"
    return "OK"
}
