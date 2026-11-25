package functiondata

fun consume(value: Handlers): Int {
    val (action, optional) = value
    val copied = value.copy(optional = optional)
    val copiedAction = copied.action
    return action(7) + copiedAction(11)
}
