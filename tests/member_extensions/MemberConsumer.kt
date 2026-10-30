fun consume(value: Host): String = with(value) { "O".decorate() }
fun distinct(left: Host, right: Host): Boolean = with(left) { right.same() }
fun nullable(value: Host): Int = with(value) { null.sizeOrZero() }
