package friends.consumer
import friends.library.Plain
private typealias Hidden = Plain
private fun read(value: Hidden): Int = value.value
fun privateAlias(): Int = read(Plain(2))
