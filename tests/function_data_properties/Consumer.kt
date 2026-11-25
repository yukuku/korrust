package functiondata

fun consume(value: Handlers): Int {
    val (action, optional) = value
    val copied = value.copy(optional = optional)
    return action(7) + copied.action(11)
}
