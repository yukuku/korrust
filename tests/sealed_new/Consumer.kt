import sealednew.*
fun consume(value: Choice): String = when(value) {
    is Choice.First -> value.value.toString()
    is Choice.Second -> value.text
}
fun consumeSignal(value: Signal?): String = when(value) {
    is Started -> "start"
    is Stopped -> "stop"
    is SetValue -> value.number.toString()
    null -> "null"
}
