package sealednew
sealed class Choice {
    class First(val value: Int) : Choice()
    class Second(val text: String) : Choice()
}
fun choose(value: Choice): String = when (value) {
    is Choice.First -> value.value.toString()
    is Choice.Second -> value.text
}
sealed interface Signal
class Started : Signal
class Stopped : Signal
fun signal(value: Signal?): String = when (value) {
    is Started -> "start"
    is Stopped -> "stop"
    is SetValue -> value.number.toString()
    null -> "null"
}
sealed class Tree
sealed class Branch : Tree()
class Left : Branch()
class Right : Branch()
class Tip : Tree()
fun tree(value: Tree): String = when (value) {
    is Branch -> "branch"
    is Tip -> "tip"
}
fun negative(value: Choice): String = when (value) {
    !is Choice.First -> "second"
    is Choice.First -> "first"
}
sealed interface Action {
    object Begin : Action
    object End : Action
}
enum class Phase : Action { UP, DOWN }
fun action(value: Action): Int = when(value) {
    Action.Begin -> 1
    Action.End -> 2
    Phase.UP -> 3
    Phase.DOWN -> 4
}
fun bool(value: Boolean?): Int = when(value) {
    true -> 1
    false -> 0
    null -> -1
}
fun statement(value: Choice): Int {
    when(value) {
        is Choice.First -> return value.value
        is Choice.Second -> return value.text.length
    }
}
fun assigned(value: Choice): Int {
    var answer: Int
    when(value) {
        is Choice.First -> answer = value.value
        is Choice.Second -> answer = value.text.length
    }
    return answer
}
fun throwing(value: Choice): String = when(value) {
    is Choice.First -> throw IllegalArgumentException("first")
    is Choice.Second -> value.text
}
sealed class Packet(val label: String, val count: Long)
class Payload(label: String, count: Long) : Packet(label, count)
sealed class Outcome {
    data class Ready(val text: String) : Outcome()
    data class Failed(val code: Int) : Outcome()
}
fun outcome(value: Outcome): String = when(value) {
    is Outcome.Ready -> value.text
    is Outcome.Failed -> value.code.toString()
}
data class SetValue(val number: Int) : Signal
fun allSignal(value: Signal): Int = when(value) {
    is Started -> 1
    is Stopped -> 2
    is SetValue -> value.number
}
fun nullableResult(value: Choice): String? = when(value) {
    is Choice.First -> null
    is Choice.Second -> value.text
}
fun observed(value: Choice): Choice {
    Probe.observe()
    return value
}
fun effects(value: Choice): Int = when(val selected = observed(value)) {
    is Choice.First -> Probe.mark(selected.value)
    is Choice.Second -> Probe.mark(selected.text.length)
}
fun ordinaryStatement(value: Int): Int {
    var result = 0
    when(value) { 1 -> result = 7 }
    return result
}
fun subjectlessStatement(value: Boolean): Int {
    var result = 0
    when { value -> result = 9 }
    return result
}
fun nullableTypeTest(value: Started?): Int = when(value) {
    is Started? -> 1
}
sealed interface Operation { fun apply(value: Int): Int }
class Increment(val step: Int) : Operation { override fun apply(value: Int): Int = value + step }
class Scale(val factor: Int) : Operation { override fun apply(value: Int): Int = value * factor }
fun dispatch(operation: Operation, value: Int): Int = operation.apply(value)
sealed class AbstractBase { abstract fun value(): Int }
class Concrete : AbstractBase() { override fun value(): Int = 7 }
fun fromBase(value: AbstractBase): Int = value.value()
fun markSignal(value: Signal) {
    when(value) {
        is Started -> Probe.mark(1)
        is Stopped -> Probe.mark(2)
        is SetValue -> Probe.mark(value.number)
    }
}
