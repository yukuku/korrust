package whenjoins
class Left
class Right
fun choose(value: Boolean): Any = when (value) {
    true -> Left()
    else -> Right()
}
